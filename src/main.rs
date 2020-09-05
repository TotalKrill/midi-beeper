use nom_midi::*;

use std::fs::File;
use std::io::prelude::*;

use rodio::Sink;
mod tone;

use anyhow;
use tone::Tone;

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "options")]
struct Opts {
    #[structopt(short, long, default_value = "1")]
    /// The time each delta is in milliseconds, must be larger than zero
    delta_ms: f32,

    #[structopt(short, long, default_value = "1")]
    /// Speed of the play
    speed: f32,

    #[structopt(short, long, default_value = "0")]
    /// Which track to use for the files
    track: usize,

    #[structopt(short, long)]
    /// from which note number should we play?
    from_note: Option<usize>,

    #[structopt(short, long)]
    /// to which note number should we play?
    until_note: Option<usize>,

    #[structopt(name = "MIDI_FILE")]
    file: String,
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

    println!("header: {:?}", midi.header);

    // the length of each delta_time in ms
    // let delta_ms = match midi.header.division {
    //     Division::Metrical(number) => 1000.0 / number as f32,
    //     Division::Timecode { fps, res } => {
    //         let fps: u8 = fps.into();

    //         (1.0 / fps as f32) / res as f32
    //     }
    // };

    let delta_ms = opt.delta_ms;

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
        println!("total time: {} ms", current_time as f32 * delta_ms);
        println!("notes: {}", changes.len());

        // start playing
        let speed = opt.speed;

        let device = rodio::default_output_device().unwrap();
        let sink = Sink::new(&device);

        let mut prevtime = 0;
        let mut prevkeys: Option<HashSet<u8>> = None;

        let mut notenum = 0;

        //let mut outvec = Vec::new();
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

                    let source = Tone::new(freq, Duration::from_millis(duration as u64));
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
                let freqs: Vec<u32> = freqs.into_iter().map(|f| f.round() as u32).collect();
                let maxf = freqs.into_iter().max().unwrap_or(0);
                let source = Tone::new(maxf as f32, Duration::from_millis(duration as u64));
                mixer_ctl.add(source);

                if duration > 1.0 {
                    // let text = format!("{},\t{:?}", duration, maxf);
                    // println!("{}", text);
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
        sink.sleep_until_end();
    }

    Ok(())
}
