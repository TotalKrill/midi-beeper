# Midi Beeper

This is a simplistic tool to try and create rust formatted melodies from MIDI files. What it means
is that it will help in creating (duration, frequency) tables of tones, so that they can be played
on the simplest way in embedded systems.

If you want to play more advanced sounds on such system, it is better to just use WAV files and
play those on some kind of speaker instead. Advanced files usually fails in this tool, since the song

## Installing or building

On ubuntu, libasound2-dev is needed

    apt-get install libasound2-dev


## Usage

Many midi files are quite advanced, this tool is not. So to get a sound we will use the trial and
error method. MIDI files contains many "tracks" of sound often. This tool will only use one of them,
set with the "--track" flag wich defaults to 0.

The speed variable is also used to set the playback speed, since this tool does not handle all the
tempochanges that the MIDI files uses, use -s/--speed

example usage to create a usable rust array from a .mid file with odd speeds.


    midi-beeper midi_file/mario.mid --speed 7 --track 2 -u 82 -q --output mario.rs

