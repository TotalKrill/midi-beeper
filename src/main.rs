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
    delta_ms: f32,
    #[structopt(short, long, default_value = "0")]
    track: usize,

    #[structopt(name = "MIDI_FILE")]
    file: String,
}

fn mid_to_freq(d: usize) -> f32 {
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
    let delta_ms = match midi.header.division {
        Division::Metrical(number) => 1000.0 / number as f32,
        Division::Timecode { fps, res } => {
            let fps: u8 = fps.into();

            (1.0 / fps as f32) / res as f32
        }
    };

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
        //let ms = 60.0 / 200.0;
        println!("total time: {}", current_time as f32 * delta_ms);

        // time taken from the mario video in seconds, so might not be very exact
        //let units_per_second = current_time as f32 / 176.0;
        // let units_per_second = ;
        // let each_unit_ms = units_per_second / 1000.0;
        // println!("units: {}", units_per_second);
        // println!("each: {}", each_unit_ms);

        // start playing
        let device = rodio::default_output_device().unwrap();
        let sink = Sink::new(&device);

        let mut prevtime = 0;
        let mut prevkeys: Option<HashSet<u8>> = None;
        for (time, keys) in changes {
            println!("time: {}, notes: {:?}", time, keys);
            let (mixer_ctl, mixer) = rodio::dynamic_mixer::mixer(1, 48000);

            let duration = time - prevtime;
            //let duration = duration * each_unit_ms.round() as u32;
            let duration = duration as f32 * delta_ms;

            //println!("duration in ms: {}", duration);

            if let Some(keys) = prevkeys {
                use std::time::Duration;
                prevtime = time;
                if keys.is_empty() {
                    let source = Tone::new(0.0, Duration::from_millis(duration as u64));
                    mixer_ctl.add(source);
                }
                for key in &keys {
                    let f = mid_to_freq(key.clone().into());
                    let source = Tone::new(f, Duration::from_millis(duration as u64));
                    mixer_ctl.add(source);
                }
                sink.append(mixer);
            }
            prevkeys = Some(keys);
        }
        sink.sleep_until_end();
    }

    Ok(())
}
