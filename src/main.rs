use nom_midi::*;

use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

use rodio::Sink;
mod tone;
use log::*;

use anyhow;
use tone::Tone;

use structopt::StructOpt;

// enum OutputFormat {
//     Csv,
//     Rust,
//     C,
// }

#[derive(StructOpt, Debug)]
#[structopt(name = "options")]
struct Opts {
    #[structopt(short, long, default_value = "1")]
    /// This is a multiplier of the playback speed,
    speed: f32,

    #[structopt(short, long, default_value = "0")]
    /// Midi files contains many tracks, this tool will only act on a single one, which track to use for the files
    track: usize,

    #[structopt(short, long)]
    /// from which note number should we play?
    from_note: Option<usize>,

    #[structopt(short, long)]
    /// to which note number should we play?
    until_note: Option<usize>,

    #[structopt(name = "MIDI_FILE", parse(from_os_str))]
    file: PathBuf,

    /// Output file, if not specified no output will be shown
    #[structopt(long, parse(from_os_str))]
    output: Option<PathBuf>,

    #[structopt(short)]
    quiet: bool,
}

fn mid_to_freq(d: u8) -> f32 {
    let two = 2.0f32;
    let freq = two.powf((d as f32 - 69.0) / 12.0) * 440.0;
    freq
}

fn main() -> anyhow::Result<()> {
    let opt = Opts::from_args();

    let mut f = File::open(opt.file)?;
    //let midi = include_bytes!("../midi/mario.mid");
    let mut midi = Vec::new();
    // read the whole file
    f.read_to_end(&mut midi)?;

    let (_, midi) = parser::parse_smf(&midi).expect("Could not parse midi file");

    debug!("header: {:?}", midi.header);

    // the length of each delta_time in ms
    // let delta_ms = match midi.header.division {
    //     Division::Metrical(number) => 1000.0 / number as f32,
    //     Division::Timecode { fps, res } => {
    //         let fps: u8 = fps.into();

    //         (1.0 / fps as f32) / res as f32
    //     }
    // };

    let delta_ms = 1.0;

    if let Some(track) = midi.tracks.get(opt.track) {
        use std::collections::HashSet;

        // we want to store when the note is on, and then, when the note is off
        let mut keystate: HashSet<u8> = HashSet::new();

        let mut changes = Vec::new();

        let mut current_time = 0;
        let mut prev_time = 0;
        for evt in &track.events {
            current_time += evt.delta_time;
            // we want the midi events
            if let EventType::Midi(mevt) = evt.event {
                // if this is the first event, then we should push some silence
                if changes.is_empty() {
                    changes.push((prev_time, keystate.clone()));
                }
                match mevt.event {
                    MidiEventType::NoteOn(evtnote, _) => {
                        // note is now off
                        keystate.insert(evtnote.into());

                        if evt.delta_time == 0 {
                            changes.pop();
                        }

                        changes.push((current_time, keystate.clone()));
                        //println!("time: {}, notes: {:?}", current_time, keystate);
                    }
                    MidiEventType::NoteOff(evtnote, _) => {
                        // note is now off
                        keystate.remove(&evtnote.into());
                        if evt.delta_time == 0 {
                            // revert previous change
                            changes.pop();
                        }
                        changes.push((current_time, keystate.clone()));
                        //println!("time: {}, notes: {:?}", current_time, keystate);
                    }
                    _ => {} //ignore most events
                }
            }
            prev_time = current_time;
        }
        let speed = opt.speed;

        println!("Midi info:");
        println!("\tTotal amount of tracks: {}", midi.tracks.len());
        println!("\tTrack {} Notes: {}", opt.track, changes.len());
        println!(
            "\tTrack {} length (speed = 1.0): {} seconds",
            opt.track,
            current_time as f32 / 1000.0
        );
        if opt.speed != 1.0 {
            println!(
                "\tTrack {} modified length (speed = {}): {} seconds",
                opt.track,
                opt.speed,
                current_time as f32 / 1000.0 / speed
            );
        }

        // start playing

        let device = rodio::default_output_device().unwrap();
        let sink = Sink::new(&device);

        let mut prevtime = 0;
        let mut prevkeys: Option<HashSet<u8>> = None;

        let mut notenum = 0;

        let mut melody = Vec::new();
        for (time, keys) in changes {
            if let Some(from_note) = opt.from_note {
                // Do nothing until we are at a larger number
                if notenum < from_note {
                    notenum += 1;
                    prevkeys = Some(keys);
                    prevtime = time;
                    continue;
                }
            }

            //println!("time: {}, notes: {:?}", time, keys);
            let (mixer_ctl, mixer) = rodio::dynamic_mixer::mixer(1, 48000);

            let duration = time - prevtime;
            //let duration = duration * each_unit_ms.round() as u32;
            let duration = duration as f32 * delta_ms / speed;

            //println!("duration in ms: {}", duration);

            if let Some(keys) = prevkeys {
                use std::time::Duration;
                if keys.is_empty() {
                    let source = Tone::new(0.0, Duration::from_millis(duration as u64));
                    mixer_ctl.add(source);
                }

                let mut freqs = Vec::new();
                for key in &keys {
                    let key: u8 = key.clone().clone();
                    let freq = mid_to_freq(key);
                    freqs.push(freq);

                    // let source = Tone::new(freq, Duration::from_millis(duration as u64));
                    //mixer_ctl.add(source);
                    //break;
                }

                // // Trying averaging the frequencies
                // let f = if !freqs.is_empty() {
                //     let sum: f32 = freqs.iter().sum();
                //     let num = freqs.len();
                //     sum / num as f32
                // } else {
                //     0.0
                // };

                // Pick out the max frequency if there are multiple, dont mix them
                let freqs: Vec<u32> = freqs.into_iter().map(|f| f.round() as u32).collect();
                let maxf = freqs.into_iter().max().unwrap_or(0);

                let source = Tone::new(maxf as f32, Duration::from_millis(duration as u64));
                mixer_ctl.add(source);

                if duration > 1.0 {
                    // let text = format!("{},\t{:?}", duration, maxf);
                    // println!("{}", text);
                    melody.push((duration as u32, maxf));

                    sink.append(mixer);
                }

                if let Some(until_note) = opt.until_note {
                    // Abort when we have passed the larger number
                    if notenum > until_note {
                        break;
                    }
                }
            }
            notenum += 1;
            prevkeys = Some(keys);
            prevtime = time;
        }
        debug!("{:#?}", melody);

        let mut total_duration = 0;
        for tone in &melody {
            total_duration += tone.0;
        }
        println!("");
        println!("Melody length and time with current settings:",);
        println!("\ttotal time: {} seconds", total_duration as f32 / 1000.0);
        println!("\tnotes:      {}", melody.len());

        if opt.quiet {
            println!("-q flag set, skipping playing the sound");
        } else {
            sink.sleep_until_end();
        }

        if let Some(outfilepath) = opt.output {
            println!("Writing to file");

            let mut f = File::create(outfilepath)?;

            let mut text = String::new();

            text.push_str(
                "/// Melody is formatted such as (duration_milliseconds, frequency_of_tone)\n",
            );

            let s = format!("pub const MELODY: [(u32, u32); {}] = [\n", melody.len());
            text.push_str(&s);

            for tone in melody {
                let s = format!("  ({},\t{}),\n", tone.0, tone.1);
                text.push_str(&s);
            }

            text.push_str("];\n");
            f.write_all(text.as_bytes())?;
        }
    }

    Ok(())
}
