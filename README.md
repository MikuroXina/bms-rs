# bms-rs

The BMS format parser.

Be-Music Source, called BMS for short, is a file format devised by Urao Yane in 1998 for a simulator of the game Beatmania by KONAMI. This describes what and when notes are arranged and its music metadata. It is a plain text file with some “command” lines starting with `#` character.

## Usage

At first, you can get the tokens stream with `lex::parse`. Then pass it and the random generator to `parse::Bms::from_token_stream` to get the notes data. Because BMS format has some randomized syntax.

```rs
use bms_rs::bms::{
    lex::parse,
    parse::{rng::RngMock, Bms},
};

let source = std::fs::read_to_string("tests/lilith_mx.bms").unwrap();
let token_stream = parse(&source).expect("must be parsed");
let rng = RngMock([1]);
let bms = Bms::from_token_stream(&token_stream, rng).expect("must be parsed");
```

## About the format

### Command

Each command starts with `#` character, and other lines will be ignored. Some commands require arguments separated by whitespace character such as spaces or tabs.

```
#PLAYER 1
#GENRE FUGA
#TITLE BAR(^^)
#ARTIST MikuroXina
#BPM 120
#PLAYLEVEL 6
#RANK 2

#WAV01 hoge.WAV
#WAV02 foo.WAV
#WAV03 bar.WAV

#00211:0303030303
```

### Header command

Header commands are used to express the metadata of the music or the definition for note arrangement.

### Message command

Message command starts with `#XXXYY:ZZ...`. `XXX` is the number of the measure, `YY` is the channel of the message, and `ZZ...` is the object id sequence.

The measure must start from 1, but some player may allow the 0 measure (i.e. Lunatic Rave 2).

The channel commonly expresses what the lane be arranged the note to.

The object id is formed by 2-digit of 36-radix (`[0-9a-zA-Z]`) integer. So the sequence length must be an even number. The 00 object id is the special id, expresses the rest (no object lies). The object lies on the position divided equally by how many the object is in the measure. For example:

```
#00211:0303000303
```

This will be placed as:

```
003|--|--------------|
   |  |03            |
   |  |03            |
   |  |              |
   |  |03            |
002|--|03------------|
   |  |  []  []  []  |
   |()|[]  []  []  []|
   |-----------------|
```
