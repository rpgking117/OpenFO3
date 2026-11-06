//! Lines people say without the dialogue menu (`world::social`): greetings
//! to the player, idle chatter, the lines of conversations between two
//! people, and "Say To" dialogue packages. The game says these through its
//! GREET procedure (`008dbe30`) and a conversation's playback (`009ee0a0`):
//! each line in its speaker's voice, one response after another, with its
//! lip sync (`faces`); its result scripts run as it starts and after it's
//! said, as the dialogue's do. They're printed, and shown on the HUD with the
//! game's General Subtitles option (`[GamePlay] bGeneralSubtitles`, off by
//! default): `world::subtitles`.
//!
//! Voices fade with the speaker's distance (`faces::place_voices`).

use std::collections::HashMap;

use bevy::audio::AudioPlayer;
use bevy::prelude::*;
use esm::FormId;
use world::dialogue::{self, Info, Speaker, PLAYER_REF};
use world::scripting::Runner;

use crate::dialogue::DialogueState;
use crate::scripts::Scripts;
use crate::GameFiles;

/// A line someone is to say.
#[derive(Debug, Clone)]
pub struct LineRequest {
    pub speaker: FormId,
    pub listener: FormId,
    pub info: Info,
    /// Said in a conversation, with whether its says force a tree request
    /// for the speaker (`world::talk_idles::conversation_forces_tree`);
    /// none for the GREET procedure's lines (greetings, chatter, Say To),
    /// which force one unless the speaker's package has idles
    /// (`world::talk_idles::greet_forces_tree`).
    pub conversation: Option<bool>,
    /// A GREET line said to its listener (a greeting, a "Say To"): while
    /// it's said the speaker's greeting timer is held at `fAIGreetingTimer`
    /// and, said to the player, the player counts as spoken to (`008dbe30`
    /// with a listener: process +0x330, player +0x6cc). Idle chatter has
    /// no listener in the game (`008eeec0` passes none).
    pub to_listener: bool,
    /// Said by a script (`Say`, `SayTo`) about this topic: the speaker's `SayToDone` blocks run
    /// when it's over ([`ScriptedLines`]).
    pub say_topic: Option<FormId>,
    /// `Say`'s or `SayTo`'s flag (`world::subtitles`).
    pub forced: bool,
}

/// The HUD's lines (`world::subtitles`), and `--subtitles`: the option on for
/// a run, without touching the player's own settings.
#[derive(Resource, Default)]
pub struct Subtitles {
    pub lines: world::subtitles::Subtitles,
    pub always: bool,
}

/// One line a script asked for.
#[derive(Debug, Clone, Copy)]
pub struct ScriptedLine {
    pub speaker: FormId,
    /// `SayTo`'s listener; 0 for `Say`, which speaks to no one in particular.
    pub to: FormId,
    /// `None`: their greeting.
    pub topic: Option<FormId>,
    /// `Say`'s or `SayTo`'s flag.
    pub forced: bool,
}

/// Lines scripts asked for this frame (`Say`, `SayTo`), read from Fallout3.exe 1.7.0.3: `Cmd_Say`
/// 00534210 and `Cmd_SayTo` 00534660 make the same call on the speaker's own process
/// (`HighProcess` vtable slot 0xa8, 00732110), which keeps that person's line. So everyone speaks
/// independently, and a new line replaces only the speaker's own. CG00's parents talk at once
/// ("Both start talking in this section"), which one shared slot lost.
#[derive(Resource, Default)]
pub struct ScriptedLines(pub Vec<ScriptedLine>);

/// A response begun this frame (each one of a line is its own say,
/// `008a20d0`): who says it to whom, and how its caller asks for idles.
#[derive(Debug, Clone)]
pub struct ResponseStarted {
    pub speaker: FormId,
    pub listener: FormId,
    pub response: world::dialogue::Response,
    pub conversation: Option<bool>,
}

/// A line being said: the response and since when (seconds), its voice.
struct Saying {
    info: Info,
    listener: FormId,
    conversation: Option<bool>,
    to_listener: bool,
    say_topic: Option<FormId>,
    forced: bool,
    response: usize,
    since: f32,
    voice: Option<Entity>,
}

/// Lines said without the dialogue menu: those to start, those being said
/// (by speaker), and those that ended this frame.
#[derive(Resource, Default)]
pub struct Lines {
    pub queue: Vec<LineRequest>,
    saying: HashMap<FormId, Saying>,
    pub done: Vec<(FormId, FormId)>,
    /// The responses begun this frame (for the speakers' and listeners'
    /// gestures, `sitting::idle_requests`).
    pub started: Vec<ResponseStarted>,
}

impl Lines {
    /// Whether someone is saying a line now.
    pub fn is_saying(&self, who: FormId) -> bool {
        self.saying.contains_key(&who) || self.queue.iter().any(|r| r.speaker == who)
    }

    /// Asks someone to say a line with no listener of its own (idle
    /// chatter; `listener` is only whom it's printed as said to).
    pub fn say(&mut self, speaker: FormId, listener: FormId, info: Info) {
        self.queue.push(LineRequest {
            speaker,
            listener,
            info,
            conversation: None,
            to_listener: false,
            say_topic: None,
            forced: false,
        });
    }

    /// Asks someone to say a GREET line to `listener` (a greeting, a
    /// "Say To" package's line).
    pub fn say_to(&mut self, speaker: FormId, listener: FormId, info: Info) {
        self.queue.push(LineRequest {
            speaker,
            listener,
            info,
            conversation: None,
            to_listener: true,
            say_topic: None,
            forced: false,
        });
    }

    /// Whether someone is saying (or about to say) a GREET line to a
    /// listener.
    pub fn greeting(&self, who: FormId) -> bool {
        self.saying.get(&who).is_some_and(|s| s.to_listener)
            || self.queue.iter().any(|r| r.speaker == who && r.to_listener)
    }

    /// Whether anyone is saying (or about to say) a GREET line to `whom`
    /// (for the player: +0x6cc, `world::social::GreetingCheck`).
    pub fn spoken_to(&self, whom: FormId) -> bool {
        self.saying
            .values()
            .any(|s| s.to_listener && s.listener == whom)
            || self
                .queue
                .iter()
                .any(|r| r.to_listener && r.listener == whom)
    }

    /// Asks someone to say a conversation's line (`009ee0a0`), its says
    /// forcing a tree request or not (`force_tree`).
    pub fn say_in_conversation(
        &mut self,
        speaker: FormId,
        listener: FormId,
        info: Info,
        force_tree: bool,
    ) {
        self.queue.push(LineRequest {
            speaker,
            listener,
            info,
            conversation: Some(force_tree),
            to_listener: false,
            say_topic: None,
            forced: false,
        });
    }

    /// Discard lines from the world that was just replaced. Their result
    /// scripts must not run against the restored game state.
    pub fn discard_pending(&mut self) {
        self.queue.clear();
        self.saying.clear();
        self.done.clear();
        self.started.clear();
    }
}

/// How long a response stays without a voice: the dialogue's reading pace
/// (a guess there too).
fn reading_time(text: &str) -> f32 {
    (text.split_whitespace().count() as f32 * 0.35).max(2.0)
}

/// A response's voice, if its file is found, with its lip sync.
fn play_voice(
    commands: &mut Commands,
    audio: &mut Assets<crate::sounds::PcmSound>,
    game: &cellview::Game,
    speaker: FormId,
    info: &Info,
    response: usize,
) -> (Option<Entity>, Option<f32>) {
    let played = (|| {
        let r = info.responses.get(response)?;
        let base = world::scripting::base_of(&game.order, speaker)?;
        let voice = Speaker::load(&game.order, speaker, base)?.voice?;
        let path = dialogue::voice_path(&game.order, info, r, voice)?;
        let bytes = game.assets.read(&path).ok()??;
        let source = crate::sounds::voice_handle(&path, &bytes, audio)?;
        let seconds = audio.get(&source).map(crate::sounds::PcmSound::seconds);
        let (settings, voice) = crate::faces::voice_playback(game, &path, speaker);
        Some((
            commands.spawn((AudioPlayer(source), settings, voice)).id(),
            seconds,
        ))
    })();
    match played {
        Some((entity, seconds)) => (Some(entity), seconds),
        None => (None, None),
    }
}

/// Whose line it is, for its priority (`00649e40`).
fn said_by(speaker: FormId) -> world::subtitles::Said {
    if speaker == PLAYER_REF {
        world::subtitles::Said::ByPlayer
    } else {
        world::subtitles::Said::BySomeone
    }
}

/// Where a line is said from: the speaker where they stand, else where the
/// state has them (a talking activator), else the player.
pub(crate) fn speaker_place(
    game: &cellview::Game,
    state: &world::scripting::GameState,
    talkers: &crate::dialogue::Talkers,
    speaker: FormId,
    feet: [f32; 3],
) -> [f32; 3] {
    if speaker == PLAYER_REF {
        return feet;
    }
    talkers
        .0
        .iter()
        .find(|t| t.reference == speaker)
        .map(|t| t.position)
        .or_else(|| state.place(&game.order, speaker).map(|(_, _, at, _)| at))
        .unwrap_or(feet)
}

/// Whether someone not loaded here still says a scripted line (`SayTo`,
/// `00534660`): a living persistent reference (it always has a process).
pub(crate) fn says_out_of_view(
    order: &esm::LoadOrder,
    state: &world::scripting::GameState,
    speaker: FormId,
) -> bool {
    !state.dead.contains(&speaker)
        && order
            .get(speaker)
            .is_some_and(|r| r.entry.header.flags & esm::flags::PERSISTENT != 0)
}

/// Runs one of a line's result scripts on its speaker.
fn run_script(
    game: &cellview::Game,
    scripts: &Scripts,
    state: &mut world::scripting::GameState,
    source: Option<&str>,
    speaker: FormId,
) {
    let Some(source) = source.filter(|s| {
        s.lines()
            .any(|l| !l.split(';').next().unwrap_or("").trim().is_empty())
    }) else {
        return;
    };
    let flow = Runner::new(&game.order, &scripts.0, state).run_source(
        source,
        Some(speaker),
        Some(speaker),
    );
    if flow == world::scripting::Flow::Stopped {
        println!("  (a line's result script stopped: it needs a function not carried out yet)");
    }
}

/// Every frame: queued lines start (said once, their first result script
/// run, the first response's voice), responses follow one another as their
/// voices end (or after a reading pause without one), and a line that's
/// over runs its second result script and is reported in `done`. Whoever
/// says a line counts as talking (`IsTalking`).
#[allow(clippy::too_many_arguments)]
pub fn say_lines(
    mut commands: Commands,
    time: Res<Time>,
    game: Res<GameFiles>,
    scripts: Res<Scripts>,
    mut state: ResMut<DialogueState>,
    mut lines: ResMut<Lines>,
    mut audio: ResMut<Assets<crate::sounds::PcmSound>>,
    voices: Query<(), With<AudioPlayer<crate::sounds::PcmSound>>>,
    (mut scripted, talkers): (ResMut<ScriptedLines>, Res<crate::dialogue::Talkers>),
    (mut subtitles, player, game_settings): (
        ResMut<Subtitles>,
        Res<crate::walk::Player>,
        Res<crate::game_menus::start::GameSettings>,
    ),
) {
    let now = time.elapsed_secs();
    let lines = &mut *lines;
    lines.done.clear();
    lines.started.clear();
    let order = &game.0.order;
    let feet = player.character.feet;
    let show = subtitles.always
        || crate::game_menus::start::general_subtitles(&game_settings, &game.0);
    let per_character = world::scripting::game_setting(order, "fNoticeTextTimePerCharacter")
        .unwrap_or(world::subtitles::SECONDS_PER_CHARACTER);
    for line in std::mem::take(&mut scripted.0) {
        let Some(base) = world::scripting::base_of(order, line.speaker) else {
            continue;
        };
        // Only someone here speaks: a talking activator has no talker entry.
        let here = talkers.0.iter().any(|t| t.reference == line.speaker)
            || order
                .get(base)
                .is_some_and(|r| r.entry.header.kind.as_bytes() == b"TACT");
        // Someone out of the loaded area: Fallout3.exe's `SayTo` (`00534660`)
        // says nothing only for someone without a process (+0x60) or dead;
        // a persistent person always has one (their low process) and says
        // the line there, its result scripts running (MQ01's Lyons' Pride
        // scene waits on them: Sarah's line to Colvin set the next turn).
        if !here {
            if !says_out_of_view(order, &state.0, line.speaker) {
                println!("A script asks {} to speak, who isn't loaded here.", line.speaker);
                continue;
            }
            println!(
                "A script asks {} to speak out of the loaded area: their process says it.",
                line.speaker
            );
        }
        // `Cmd_SayTo` first stops what they're saying (00765ee0), and that marks the stopped
        // line's `SayToDone` event (mask 0x40000), so a script waiting on it doesn't hang.
        if line.to.0 != 0 {
            if let Some(old) = lines.saying.get(&line.speaker).and_then(|s| s.say_topic) {
                Runner::new(order, &scripts.0, &mut state.0).say_to_done(line.speaker, old);
            }
        }
        hush(&mut commands, lines, line.speaker);
        let topic = line.topic.unwrap_or(crate::dialogue::GREETING);
        state.0.roll();
        let info = Speaker::load(order, line.speaker, base)
            .and_then(|speaker| dialogue::pick(order, topic, &speaker, &state.0));
        match info {
            Some(info) => lines.queue.push(LineRequest {
                speaker: line.speaker,
                listener: if line.to.0 == 0 { PLAYER_REF } else { line.to },
                info,
                conversation: None,
                to_listener: line.to.0 != 0,
                say_topic: Some(topic),
                forced: line.forced,
            }),
            // 00732110: with nothing to say the event is marked at once.
            None => {
                Runner::new(order, &scripts.0, &mut state.0).say_to_done(line.speaker, topic);
            }
        }
    }
    for request in std::mem::take(&mut lines.queue) {
        if lines.saying.contains_key(&request.speaker) {
            continue;
        }
        let info = request.info;
        state.0.said.insert(info.form_id);
        let said: Vec<&str> = info.responses.iter().map(|r| r.text.as_str()).collect();
        println!(
            "{:.1} s: {} says to {} ({}): {}",
            now,
            request.speaker,
            request.listener,
            info.form_id,
            said.join(" ")
        );
        run_script(
            &game.0,
            &scripts,
            &mut state.0,
            info.begin_script.as_deref(),
            request.speaker,
        );
        let (voice, voice_seconds) = play_voice(
            &mut commands,
            &mut audio,
            &game.0,
            request.speaker,
            &info,
            0,
        );
        if show {
            if let Some(r) = info.responses.first() {
                let at = speaker_place(&game.0, &state.0, &talkers, request.speaker, feet);
                subtitles.lines.add(
                    &r.text,
                    world::subtitles::seconds_of(&r.text, voice_seconds, per_character),
                    at,
                    said_by(request.speaker),
                    request.forced,
                    f64::from(now) * 1000.0,
                );
            }
        }
        if let Some(r) = info.responses.first() {
            lines.started.push(ResponseStarted {
                speaker: request.speaker,
                listener: request.listener,
                response: r.clone(),
                conversation: request.conversation,
            });
        }
        lines.saying.insert(
            request.speaker,
            Saying {
                info,
                listener: request.listener,
                conversation: request.conversation,
                to_listener: request.to_listener,
                say_topic: request.say_topic,
                forced: request.forced,
                response: 0,
                since: now,
                voice,
            },
        );
    }
    subtitles.lines.update(f64::from(now) * 1000.0, feet);
    let mut over = Vec::new();
    let started = &mut lines.started;
    for (&speaker, saying) in lines.saying.iter_mut() {
        // Begun this frame: its voice is spawned by a command that hasn't run yet, so it would
        // look finished.
        if saying.since == now {
            continue;
        }
        let done = match saying.voice {
            Some(v) => voices.get(v).is_err(),
            None => {
                let text = saying
                    .info
                    .responses
                    .get(saying.response)
                    .map_or("", |r| r.text.as_str());
                now - saying.since > reading_time(text)
            }
        };
        if !done {
            continue;
        }
        saying.response += 1;
        saying.since = now;
        if saying.response < saying.info.responses.len() {
            started.push(ResponseStarted {
                speaker,
                listener: saying.listener,
                response: saying.info.responses[saying.response].clone(),
                conversation: saying.conversation,
            });
            let (voice, voice_seconds) = play_voice(
                &mut commands,
                &mut audio,
                &game.0,
                speaker,
                &saying.info,
                saying.response,
            );
            saying.voice = voice;
            if show {
                let r = &saying.info.responses[saying.response];
                let at = speaker_place(&game.0, &state.0, &talkers, speaker, feet);
                subtitles.lines.add(
                    &r.text,
                    world::subtitles::seconds_of(&r.text, voice_seconds, per_character),
                    at,
                    said_by(speaker),
                    saying.forced,
                    f64::from(now) * 1000.0,
                );
            }
        } else {
            over.push(speaker);
        }
    }
    for speaker in over {
        if let Some(saying) = lines.saying.remove(&speaker) {
            run_script(
                &game.0,
                &scripts,
                &mut state.0,
                saying.info.end_script.as_deref(),
                speaker,
            );
            lines.done.push((speaker, saying.info.form_id));
            if let Some(topic) = saying.say_topic {
                Runner::new(&game.0.order, &scripts.0, &mut state.0).say_to_done(speaker, topic);
            }
        }
    }
    for &speaker in lines.saying.keys() {
        state.0.speaking.insert(speaker);
    }
}

/// A line is cut short (the speaker died, or was talked to): its voice
/// stops and the face eases back.
pub fn hush(commands: &mut Commands, lines: &mut Lines, who: FormId) {
    lines.queue.retain(|r| r.speaker != who);
    if let Some(saying) = lines.saying.remove(&who) {
        if let Some(v) = saying.voice {
            if let Ok(mut e) = commands.get_entity(v) {
                e.despawn();
            }
        }
        crate::faces::cut_short(commands, who);
    }
}
