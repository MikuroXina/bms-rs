# BMS Format Specification

written by Urao Yane <yaneurao@gmail.com>

"BMS" means a Be-Music Source file. A file which has BMS suffix is regarded as the BMS file. This file format was produced by Urao Yane and NBK in 1998. And I adopted this file format to BM98. Now,anyone can use this format freely.

---

# Command Line

The line beginning at `#` is the command line. All the rest are ignored (use for comments). And this BMS file is compiled at runtime, so you can order any lines freely. And there is no difference in the command line between using a capital letter or not.

# Header

    #PLAYER 1
This data is for Single Play.

    #PLAYER 2
This data is for Two Play.

    #PLAYER 3
This data is for Double Play.

    #GENRE xxxxxxxx
Definition of Genre.

    #TITLE xxxxxxxx
Definition of Title.

    #ARTIST xxxxxxxx
Definition of Artist.

    #BPM xxx
Definition of BPM (Beat Per Minute) at the top of music. Default is 130.

    #MIDIFILE xxxxxxx.mid
Background music by MIDI.

    #PLAYLEVEL x
Information of Game Level for player.

    #RANK x
judgement level. x = `0`: very hard, `1`: hard, `2`: normal, `3`: easy.

    #VOLWAV xxx
Relative volume control in percentage.

    #WAVxx yyyyyyyy.wav
Definition of Wave Data. `xx`: `01` to `FF` (Hex), `yyyyyyyy.wav`: wave file name.

e.g.

    #WAV01 HOUSE01.WAV // assign HOUSE01.WAV to 01 wav
    #WAV02 HOUSE02.WAV // assign HOUSE02.WAV to 02 wav
    #WAVFF HOUSE03.WAV // assign HOUSE03.WAV to FF wav

---

    #BMPxx yyyyyyyy.bmp
Definition of Bitmap file. `xx`: `01` to `FF` (Hex), `yyyyyyyy.bmp`: bitmap file name. Bitmap size must be 256 * 256. (max color 65536)

e.g.

    #BMP02 HOUSE02.BMP // assign HOUSE02.BMP to 02 bitmap
    #BMP01 HOUSE01.BMP // assign HOUSE01.BMP to 01 bitmap
    #BMPEE HOUSE03.BMP // assign HOUSE03.BMP to EE bitmap

However, the bitmap defined by `#BMP00` is something special. This bitmap shows when a player do a poor play.

## Example

    // a sample of random loading function

    #random 2 // create a random number (1 or 2)

    #if 1 // if the number was equal to 1 then...
    #00111:31313131 // this is effective...
    #endif

    #if 2 // if the number was equal to 2 then...
    #00113:32003232 // this is effective
    #endif

# Channel Messages

`#aaabb:cccccccc`

`aaa`: track number (from `000` to `999`).
`bb`: channel number where you want to send message (from `00` to `FF`).
`cccccccc`: any message.

## A brief Channel Number

`01`: BGM (background music by WAVE).
`03`: changing a Tempo.
`04`: BGA (background animation).
`06`: changing Poor-bitmap.
`11` to `17`: Object Channels of 1 player side from left to right.
`21` to `27`: Object Channels of 2 player side from left to right.

## Example

    #00211:03030303
This means 4 object `03`s at the left of 1 player side `11` in `002` track. This object is assigned to wave No. `03` which was defined by `#WAV03 xxxx.wav`. And this 4 objects are arranged evenly in this track.

Please try the following patterns.

    #00211:0303030303

    #00211:0303000303

    #00211:010101
    #00211:00020202

---

This document and this format is free! I hope the day will come when my BMS format will use all over the world.
