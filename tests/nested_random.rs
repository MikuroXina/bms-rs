use bms_rs::{lex::parse, parse::Bms};
use rand::rngs::mock::StepRng;

#[test]
fn nested_random() {
    const SRC: &str = r"
    #00111:11000000

    #RANDOM 2

      #IF 1
        #00112:00220000

        #RANDOM 2

          #IF 1
            #00115:00550000
          #ENDIF

          #IF 2
            #00116:00006600
          #ENDIF

        #ENDRANDOM

      #ENDIF

      #IF 2
        #00113:00003300
      #ENDIF

    #ENDRANDOM

    #00114:00000044";

    let ts = parse(SRC).expect("must be parsed");
    let rng = StepRng::new(0, 0);
    let bms = Bms::from_token_stream(&ts, rng).expect("must be parsed");

    eprintln!("{:?}", bms);
}
