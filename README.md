# bms-rs

The BMS format parser.

Be-Music Source, called BMS for short, is a file format devised by Urao Yane in 1998 for a simulator of the game Beatmania by KONAMI. This describes what and when notes are arranged and its music metadata. It is a plain text file with some “command” lines starting with `#` character.

## Usage

At first, you can get the tokens stream with `lex::parse`. Then pass it and the random generator to `parse::Bms::from_token_stream` to get the notes data. Because BMS format has some randomized syntax.

```rs
use bms_rs::{
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

## BMS Format Reference

- (Main) BMS command memo ([JP](https://hitkey.nekokan.dyndns.info/cmdsJP.htm)/[EN](https://hitkey.nekokan.dyndns.info/cmds.htm))
  - These below are its references.
  - [Wiki](https://en.wikipedia.org/wiki/Be-Music_Source)
  - [wiki.bms.ms(Archive)](https://web.archive.org/web/*/http://wiki.bms.ms/Bms:Spec)
  - [First Version By Urao Yane](http://bm98.yaneu.com/bm98/bmsformat.html)
  - https://nvyu.net/rdm/rby_ex.php
  - https://cosmic.mearie.org/f/sonorous/bmsexts
  - http://dtxmania.net/wiki.cgi?page=qa_dtx_spec_e
  - https://cosmic.mearie.org/2005/03/bmsguide/
  - https://github.com/lifthrasiir/angolmois/blob/master/INTERNALS.md
  - [RDM's LONG NOTE support](https://web.archive.org/web/*/http://ivy.pr.co.kr/rdm/jp/extension.htm)
  - http://right-stick.sub.jp/lr2skinhelp.html
- [Bemuse Extensions](https://bemuse.ninja/project/docs/bms-extensions/)
  - [For `#SCROLL`](https://hitkey.nekokan.dyndns.info/bmse_help_full/beat.html)
- [Base62](https://docs.google.com/document/u/0/d/e/2PACX-1vTl8zOS3ukl5HpuNsBUlN8rn_ZaNdJSHb8a4se3Z3ap9Y6UJ1nB8LA3HnxWAk9kMTDp0j9orpg43-tl/pub)
- [Beatoraja Extensions](https://github.com/exch-bms2/beatoraja/wiki/%E6%A5%BD%E6%9B%B2%E8%A3%BD%E4%BD%9C%E8%80%85%E5%90%91%E3%81%91%E8%B3%87%E6%96%99#bms%E6%8B%A1%E5%BC%B5%E5%AE%9A%E7%BE%A9)

## Supported features

For supported commands, see [docs.rs#Token](https://docs.rs/bms-rs/latest/bms_rs/bms/lex/token/enum.Token.html).

For supported note channels, see [docs.rs#Channel](https://docs.rs/bms-rs/0.7.0/bms_rs/bms/lex/command/enum.Channel.html).