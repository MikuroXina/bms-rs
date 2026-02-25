# Changelog

## [1.0.0](https://github.com/MikuroXina/bms-rs/compare/v0.10.1...v1.0.0) (2026-02-25)


### ⚠ BREAKING CHANGES

* **bms:** rm fraction, use StringValue for `FromStr` values ([#331](https://github.com/MikuroXina/bms-rs/issues/331))
* **chart_processor:** split player ([#295](https://github.com/MikuroXina/bms-rs/issues/295))
* **chart_processor:** `visible_events` now return RangeInclusive; fix formula ([#287](https://github.com/MikuroXina/bms-rs/issues/287))
* **bms::parse:** impl new control flow structure, let `Bms` contain all branches ([#268](https://github.com/MikuroXina/bms-rs/issues/268))
* **chart_processor:** add VisibleRangePerBpm & replace SystemTime with Instant ([#271](https://github.com/MikuroXina/bms-rs/issues/271))
* replace use_relaxed with TokenModifier ([#263](https://github.com/MikuroXina/bms-rs/issues/263))
* remove seperate of common_preset/minor_preset ([#262](https://github.com/MikuroXina/bms-rs/issues/262))
* use Result<Bms, ParseError> in BmsOutput/ParseOutput, functions return BmsOutput/ParseOutput again ([#251](https://github.com/MikuroXina/bms-rs/issues/251))
* use id to sign different events, avoid duplicate calculation ([#249](https://github.com/MikuroXina/bms-rs/issues/249))

### Features

* add activate_time ([#259](https://github.com/MikuroXina/bms-rs/issues/259)) ([ca38d1f](https://github.com/MikuroXina/bms-rs/commit/ca38d1fc2a7ab1dda78f05bd864e0152d962dcff))
* add collect_bms_reports not to print extra diagnostic in tests ([#254](https://github.com/MikuroXina/bms-rs/issues/254)) ([56761b2](https://github.com/MikuroXina/bms-rs/commit/56761b2252c73dc3e6ca1584704eb52971e36468))
* **bms::parse:** impl new control flow structure, let `Bms` contain all branches ([#268](https://github.com/MikuroXina/bms-rs/issues/268)) ([cabde97](https://github.com/MikuroXina/bms-rs/commit/cabde97b9ebc14932d43446c3d5a11d6e5ff2d9b))
* **bms:** allow #XXX02:0.0 ([#311](https://github.com/MikuroXina/bms-rs/issues/311)) ([4417c82](https://github.com/MikuroXina/bms-rs/commit/4417c82952dc027280df755f36d75116dcad9cb7))
* **chart_processor:** `visible_events` now return RangeInclusive; fix formula ([#287](https://github.com/MikuroXina/bms-rs/issues/287)) ([430c7b6](https://github.com/MikuroXina/bms-rs/commit/430c7b6161c1d4ffb3ba7d9dcac01d82fd971b37))
* **chart_processor:** add started_at ([#270](https://github.com/MikuroXina/bms-rs/issues/270)) ([ac0e295](https://github.com/MikuroXina/bms-rs/commit/ac0e295bb1129d07cbd7fde9a9aa2ac2673888d6))
* **chart_processor:** add VisibleRangePerBpm & replace SystemTime with Instant ([#271](https://github.com/MikuroXina/bms-rs/issues/271)) ([5876197](https://github.com/MikuroXina/bms-rs/commit/5876197d929244015560b5a5666ae5c10d7d7160))
* **chart_processor:** create player at start ([#297](https://github.com/MikuroXina/bms-rs/issues/297)) ([9af19a4](https://github.com/MikuroXina/bms-rs/commit/9af19a455cfdff00294babcb55517f9c175dd9ba))
* **chart_processor:** events_in_time_range ([#273](https://github.com/MikuroXina/bms-rs/issues/273)) ([ad90779](https://github.com/MikuroXina/bms-rs/commit/ad90779b9adf7e6f4f124e82365c7b413bd1dec6))
* **chart_processor:** split player ([#295](https://github.com/MikuroXina/bms-rs/issues/295)) ([66cda85](https://github.com/MikuroXina/bms-rs/commit/66cda856256810bd461b8bc90abf6f35dd18aa0a))
* compute continue duration ([#255](https://github.com/MikuroXina/bms-rs/issues/255)) ([c6ed199](https://github.com/MikuroXina/bms-rs/commit/c6ed19985d5b353231bbcaa96e08e4b310cb47cd))
* not binding to original chart object ([#269](https://github.com/MikuroXina/bms-rs/issues/269)) ([21e9387](https://github.com/MikuroXina/bms-rs/commit/21e93877bf967236e55b120d2af55e800e4e9ff2))
* **processor:** Impl `TryFrom` ([#328](https://github.com/MikuroXina/bms-rs/issues/328)) ([2531e86](https://github.com/MikuroXina/bms-rs/commit/2531e86f7851716e34b47173b5e34df48136dcd7))
* Random/Switch model ([#253](https://github.com/MikuroXina/bms-rs/issues/253)) ([060c1ae](https://github.com/MikuroXina/bms-rs/commit/060c1ae9f253d005070dd5d85e35090667b8dc22))
* replace use_relaxed with TokenModifier ([#263](https://github.com/MikuroXina/bms-rs/issues/263)) ([bf4f4be](https://github.com/MikuroXina/bms-rs/commit/bf4f4bee63a3edb5b1f9ee7c9a6cd849b0664cc6))
* use id to sign different events, avoid duplicate calculation ([#249](https://github.com/MikuroXina/bms-rs/issues/249)) ([a3fad4a](https://github.com/MikuroXina/bms-rs/commit/a3fad4aba8499500ce881225486a721e1f496e20))
* use Result&lt;Bms, ParseError&gt; in BmsOutput/ParseOutput, functions return BmsOutput/ParseOutput again ([#251](https://github.com/MikuroXina/bms-rs/issues/251)) ([228a5b5](https://github.com/MikuroXina/bms-rs/commit/228a5b5041880fd482c8b0cc752b7521993e3ba4))


### Bug Fixes

* add lints ([#285](https://github.com/MikuroXina/bms-rs/issues/285)) ([5adaf4a](https://github.com/MikuroXina/bms-rs/commit/5adaf4a8bc63fd93315008efee7b83211e47359d))
* **bms_processor:** stop length ([#291](https://github.com/MikuroXina/bms-rs/issues/291)) ([b786ed8](https://github.com/MikuroXina/bms-rs/commit/b786ed814f85d5acd63d5632648fc5258bd1b657))
* **bmson:** use FinF64 from strict-num-extended ([#296](https://github.com/MikuroXina/bms-rs/issues/296)) ([b8640e5](https://github.com/MikuroXina/bms-rs/commit/b8640e59a9c9d4795bb2a83ed61036ece4d9276c))
* **bms:** rm fraction, use StringValue for `FromStr` values ([#331](https://github.com/MikuroXina/bms-rs/issues/331)) ([42ed441](https://github.com/MikuroXina/bms-rs/commit/42ed44172c41d4b0d4882d378d626117b2d2a85d))
* **chart_process:** FlowEvents in same Y should be triggered together ([#292](https://github.com/MikuroXina/bms-rs/issues/292)) ([03566c0](https://github.com/MikuroXina/bms-rs/commit/03566c0613490cb5b19d0313df25b7cd03eb2b6f))
* **chart_processor:** restore speed/scroll on start ([#282](https://github.com/MikuroXina/bms-rs/issues/282)) ([41b9178](https://github.com/MikuroXina/bms-rs/commit/41b9178e6052e84c260fcd8d985926f55316ba6a))
* **chart_processor:** visible_events no need passing `now` and use a new return type ([#280](https://github.com/MikuroXina/bms-rs/issues/280)) ([180c66a](https://github.com/MikuroXina/bms-rs/commit/180c66a439477932ef8d732ce6b3aa4012f4a52e))
* **chart_process:** remove nanosecond ([#336](https://github.com/MikuroXina/bms-rs/issues/336)) ([5661e40](https://github.com/MikuroXina/bms-rs/commit/5661e40514c7d361b70fe0bcd81f3a4b59a1fa3a))
* **chart_process:** use round() to avoid misalignment ([#293](https://github.com/MikuroXina/bms-rs/issues/293)) ([765ddcc](https://github.com/MikuroXina/bms-rs/commit/765ddcc5cff986f72c639d03d0d583cc3dc391cb))
* clippy with nursery ([#250](https://github.com/MikuroXina/bms-rs/issues/250)) ([6811115](https://github.com/MikuroXina/bms-rs/commit/6811115b1ee3388e9ddedaf8cb7b1f5dda0201ec))
* delete `crate::bms` pub use ([#245](https://github.com/MikuroXina/bms-rs/issues/245)) ([01f5ce5](https://github.com/MikuroXina/bms-rs/commit/01f5ce544a49828e09ae46427ddaacbb9acb0488))
* fill feature gate for compile in different features ([#240](https://github.com/MikuroXina/bms-rs/issues/240)) ([c964d53](https://github.com/MikuroXina/bms-rs/commit/c964d537c686b495efa432aeb0e3e06a920b8989))
* impl `ToAradine` for `PlayingWarning`/`PlayingError` ([#252](https://github.com/MikuroXina/bms-rs/issues/252)) ([879155e](https://github.com/MikuroXina/bms-rs/commit/879155ee75ff47fa803ae88bb100330a7da0c7cf))
* **player:** LN showing ([#326](https://github.com/MikuroXina/bms-rs/issues/326)) ([c2787df](https://github.com/MikuroXina/bms-rs/commit/c2787df3c5c4f497145a8cb05bcba2fbea8964be))
* rearrange chart_process ([#314](https://github.com/MikuroXina/bms-rs/issues/314)) ([1567fa5](https://github.com/MikuroXina/bms-rs/commit/1567fa5c561325fdf962425c0536f2260e2965af))
* refine `ObjTime` constructor ([#257](https://github.com/MikuroXina/bms-rs/issues/257)) ([9bc148c](https://github.com/MikuroXina/bms-rs/commit/9bc148cf2d870cc770398951cb3ae5bae73b6258))
* remove seperate of common_preset/minor_preset ([#262](https://github.com/MikuroXina/bms-rs/issues/262)) ([06d0727](https://github.com/MikuroXina/bms-rs/commit/06d0727a3dfdb9fa42932b1d40295329ac823ffd))
* Rename into PlayheadEvent, VisibleChartEvent ([#261](https://github.com/MikuroXina/bms-rs/issues/261)) ([9372b93](https://github.com/MikuroXina/bms-rs/commit/9372b93c09c61bf5530201b287241db60112f4fe))
* rm audio/wav files from ChartPlayer ([#305](https://github.com/MikuroXina/bms-rs/issues/305)) ([ebf63ee](https://github.com/MikuroXina/bms-rs/commit/ebf63ee9110fa65f7152016e2cd3b525041cb648))
* rm round calling ([#306](https://github.com/MikuroXina/bms-rs/issues/306)) ([7954aff](https://github.com/MikuroXina/bms-rs/commit/7954aff68a4742ea339c7fa5f3bdc7fae5527c93))
* use `AsRef` for new-type struct, and fix unneed clone/ref ([#283](https://github.com/MikuroXina/bms-rs/issues/283)) ([bd6b4ef](https://github.com/MikuroXina/bms-rs/commit/bd6b4ef1bc6e964d8cb8425f98f2ba38ae40f3e8))
* use fraction::BigDecimal as Decimal ([#260](https://github.com/MikuroXina/bms-rs/issues/260)) ([f28c578](https://github.com/MikuroXina/bms-rs/commit/f28c578fab78c136ec809c70f7d3244d7812369e))
* **visible_event:** formula ([#323](https://github.com/MikuroXina/bms-rs/issues/323)) ([26b92af](https://github.com/MikuroXina/bms-rs/commit/26b92af4b5f2ffc06ef26214223c366e619f3e8f))

## [0.10.1](https://github.com/MikuroXina/bms-rs/compare/v0.10.0...v0.10.1) (2025-10-25)


### Features

* Hotfix to Add key_mapper ([#237](https://github.com/MikuroXina/bms-rs/issues/237)) ([d4d5879](https://github.com/MikuroXina/bms-rs/commit/d4d5879eaaf3f22b3650e1c6703f9e542332d8e6))

## [0.10.0](https://github.com/MikuroXina/bms-rs/compare/v0.9.0...v0.10.0) (2025-10-25)


### ⚠ BREAKING CHANGES

* Enforce token processor output and model definition ([#233](https://github.com/MikuroXina/bms-rs/issues/233))
* Make token processors stronger and Remove ast module ([#232](https://github.com/MikuroXina/bms-rs/issues/232))
* Refresh token system ([#215](https://github.com/MikuroXina/bms-rs/issues/215))
* **bmson:** parse_bmson use chumsky, move `bms::diagnostics` to `crate::diagnostics` ([#205](https://github.com/MikuroXina/bms-rs/issues/205))
* ObjTime should use NonZeroU64 ([#207](https://github.com/MikuroXina/bms-rs/issues/207))

### Features

* Add use_pedantic/use_relaxed ([#235](https://github.com/MikuroXina/bms-rs/issues/235)) ([8a53261](https://github.com/MikuroXina/bms-rs/commit/8a5326120f28c555ae1548c60bd9a3148bf818f2))
* **bmson:** parse_bmson use chumsky, move `bms::diagnostics` to `crate::diagnostics` ([#205](https://github.com/MikuroXina/bms-rs/issues/205)) ([94a523d](https://github.com/MikuroXina/bms-rs/commit/94a523d517ebc8972c630679f330510686a19c1c))
* chart_processor ([#200](https://github.com/MikuroXina/bms-rs/issues/200)) ([a0771f8](https://github.com/MikuroXina/bms-rs/commit/a0771f8bc09f9da062f086fd972a2ba579169262))
* Enforce token processor output and model definition ([#233](https://github.com/MikuroXina/bms-rs/issues/233)) ([3516134](https://github.com/MikuroXina/bms-rs/commit/3516134efa0e55eb41f03483e706b1c5526f5182))
* Impl bms to token ([#212](https://github.com/MikuroXina/bms-rs/issues/212)) ([62d842d](https://github.com/MikuroXina/bms-rs/commit/62d842d6193e139ff99ffc87a255a31a5e795339))
* impl Channel and NoteChannelId convert ([#227](https://github.com/MikuroXina/bms-rs/issues/227)) ([6876187](https://github.com/MikuroXina/bms-rs/commit/68761876c6ad58a5f3aceebc68ba24652bfa5f9e))
* Make token processors stronger and Remove ast module ([#232](https://github.com/MikuroXina/bms-rs/issues/232)) ([106226c](https://github.com/MikuroXina/bms-rs/commit/106226c69674d5e08e3cc64f60120d46f230d0e1))
* Merge message parse logic, dealing with non-fit chars, and more recovable message parsing ([#210](https://github.com/MikuroXina/bms-rs/issues/210)) ([db4bc02](https://github.com/MikuroXina/bms-rs/commit/db4bc0291a863e3ba706218ea3693e628496cf46))
* parse_bmson, serde_path_to_error version ([#201](https://github.com/MikuroXina/bms-rs/issues/201)) ([6ab9044](https://github.com/MikuroXina/bms-rs/commit/6ab904406f30931cb8a07d141b23565fd5a75227))


### Bug Fixes

* Avoid to_ascii_uppercase ([#234](https://github.com/MikuroXina/bms-rs/issues/234)) ([92a4e79](https://github.com/MikuroXina/bms-rs/commit/92a4e7904248b2da89c58be30b4fa0b9b245c32e))
* bmson resolution deserialize ([#209](https://github.com/MikuroXina/bms-rs/issues/209)) ([da713c1](https://github.com/MikuroXina/bms-rs/commit/da713c12a2432029d9ca29006dfc3f94c844c752))
* Fix clippy nursery warnings ([#217](https://github.com/MikuroXina/bms-rs/issues/217)) ([7cc678e](https://github.com/MikuroXina/bms-rs/commit/7cc678e9c24beaefd24d2408c16a5fed90daa222))
* Hotfix ([#216](https://github.com/MikuroXina/bms-rs/issues/216)) ([7545288](https://github.com/MikuroXina/bms-rs/commit/7545288c4837f4fb8e55dbacbecfa95aeab12a70))
* ObjTime should use NonZeroU64 ([#207](https://github.com/MikuroXina/bms-rs/issues/207)) ([2711155](https://github.com/MikuroXina/bms-rs/commit/2711155345512b787abfcdff4475fe531e4bb20f))
* Refresh token system ([#215](https://github.com/MikuroXina/bms-rs/issues/215)) ([5ba3542](https://github.com/MikuroXina/bms-rs/commit/5ba3542cee296895b5c28a1839ce3039043779a0))


### Miscellaneous Chores

* Release 0.10.0 ([#236](https://github.com/MikuroXina/bms-rs/issues/236)) ([1e85ffa](https://github.com/MikuroXina/bms-rs/commit/1e85ffaf53e9897f86ffa216854d7e0860ef4c44))

## [0.9.0](https://github.com/MikuroXina/bms-rs/compare/v0.8.0...v0.9.0) (2025-09-02)


### ⚠ BREAKING CHANGES

* Change WavObj to hold channel id ([#195](https://github.com/MikuroXina/bms-rs/issues/195))
* Rename Obj into WavObj ([#192](https://github.com/MikuroXina/bms-rs/issues/192))
* Impl fancy source warnings errors ([#187](https://github.com/MikuroXina/bms-rs/issues/187))
* impl channel key bindings with Bms ([#188](https://github.com/MikuroXina/bms-rs/issues/188))
* Move out AST steps ([#164](https://github.com/MikuroXina/bms-rs/issues/164))
* Delete "extend message" impl ([#157](https://github.com/MikuroXina/bms-rs/issues/157))
* use struct-binding function for single step ([#154](https://github.com/MikuroXina/bms-rs/issues/154))
* last add tokens ([#149](https://github.com/MikuroXina/bms-rs/issues/149))
* impl From<Bmson> for Bms ([#145](https://github.com/MikuroXina/bms-rs/issues/145))
* Add position info for `Token` & `ParseWarning` ([#146](https://github.com/MikuroXina/bms-rs/issues/146))

### Features

* Add AstParseWarning ([#178](https://github.com/MikuroXina/bms-rs/issues/178)) ([7169d5c](https://github.com/MikuroXina/bms-rs/commit/7169d5c0dd7d9f044c6adfa75aec1ffc2b8fc056))
* Add NoteChannelId to prelude ([#196](https://github.com/MikuroXina/bms-rs/issues/196)) ([b6f6eeb](https://github.com/MikuroXina/bms-rs/commit/b6f6eeb0718feb1722622f99e2b4837a1c622de4))
* Add Notes API ([#193](https://github.com/MikuroXina/bms-rs/issues/193)) ([29fb89e](https://github.com/MikuroXina/bms-rs/commit/29fb89e57b117835530ce6c1bdc5612b0b9dfc30))
* add ParseWarning::UnexpectedControlFlow ([#155](https://github.com/MikuroXina/bms-rs/issues/155)) ([5655747](https://github.com/MikuroXina/bms-rs/commit/5655747e435d2e718c465a6acc44c52177e3012a))
* Add position info for `Token` & `ParseWarning` ([#146](https://github.com/MikuroXina/bms-rs/issues/146)) ([08b61ee](https://github.com/MikuroXina/bms-rs/commit/08b61ee314240ff0eaf512a66f0eb4192d944d04))
* auto-close random-block ([#175](https://github.com/MikuroXina/bms-rs/issues/175)) ([2c7d9ee](https://github.com/MikuroXina/bms-rs/commit/2c7d9ee87f938b8049f1e152a2455900b38015f8))
* Delete "extend message" impl ([#157](https://github.com/MikuroXina/bms-rs/issues/157)) ([1c2afb4](https://github.com/MikuroXina/bms-rs/commit/1c2afb445f126dd19fe71e9c655a2692dac9f02c))
* Impl ast to token ([#170](https://github.com/MikuroXina/bms-rs/issues/170)) ([872293c](https://github.com/MikuroXina/bms-rs/commit/872293ce0f1b62a5f9a26cd58d4164f178463d8a))
* Impl fancy source warnings errors ([#187](https://github.com/MikuroXina/bms-rs/issues/187)) ([3262dc0](https://github.com/MikuroXina/bms-rs/commit/3262dc04da1c43c48e3a7ca4eb31b0c8f8258655))
* impl From&lt;Bmson&gt; for Bms ([#145](https://github.com/MikuroXina/bms-rs/issues/145)) ([ed0c95f](https://github.com/MikuroXina/bms-rs/commit/ed0c95f1f74643b73858a11c6c79f6e0186735fb))
* Impl keymapping convert, add beatoraja's random & r-random impl ([#166](https://github.com/MikuroXina/bms-rs/issues/166)) ([f9f244c](https://github.com/MikuroXina/bms-rs/commit/f9f244ca06de1ab1fd046b9e4bbae2dc96b6b708))
* Impl text pos mixin ([#151](https://github.com/MikuroXina/bms-rs/issues/151)) ([0460db8](https://github.com/MikuroXina/bms-rs/commit/0460db838709fd2e2bb9f1dead5f289eb75a67ad))
* Impl token to string ([#167](https://github.com/MikuroXina/bms-rs/issues/167)) ([93f441e](https://github.com/MikuroXina/bms-rs/commit/93f441e2b905facdbef016c8a5d370a5a6bb4688))
* Impl validity for bms ([#183](https://github.com/MikuroXina/bms-rs/issues/183)) ([82df2f1](https://github.com/MikuroXina/bms-rs/commit/82df2f12ce0e07324af86f129aa71bded294c2da))
* last add tokens ([#149](https://github.com/MikuroXina/bms-rs/issues/149)) ([159fb8f](https://github.com/MikuroXina/bms-rs/commit/159fb8ffe16f56d0dd2a042d0b877dad37400a80))
* make `&tokens` able for all steps ([#163](https://github.com/MikuroXina/bms-rs/issues/163)) ([1219c78](https://github.com/MikuroXina/bms-rs/commit/1219c78839fab02875ff53d5b74d29949cd3e8e8))
* make ast structure public, make the AstParseWarning get the pos ([#182](https://github.com/MikuroXina/bms-rs/issues/182)) ([a6d288d](https://github.com/MikuroXina/bms-rs/commit/a6d288d81de89893b224d9c26642b06a5b0b42f2))
* Move out ast part ([#159](https://github.com/MikuroXina/bms-rs/issues/159)) ([a0df07b](https://github.com/MikuroXina/bms-rs/commit/a0df07b30e28db883f45b3af8d699773ebd6d72e))
* Move out AST steps ([#164](https://github.com/MikuroXina/bms-rs/issues/164)) ([f12689d](https://github.com/MikuroXina/bms-rs/commit/f12689d2226798f55129d83712021902746e7846))
* new channel convert method ([#153](https://github.com/MikuroXina/bms-rs/issues/153)) ([4a7b69d](https://github.com/MikuroXina/bms-rs/commit/4a7b69d3034e46621f23f96a39d87e633b69731f))
* Split prompt warning types ([#158](https://github.com/MikuroXina/bms-rs/issues/158)) ([b0fe15b](https://github.com/MikuroXina/bms-rs/commit/b0fe15b90d1b21c0cfc75800b8534863e22d3fa1))
* support for --no-default-features ([#177](https://github.com/MikuroXina/bms-rs/issues/177)) ([fc6d348](https://github.com/MikuroXina/bms-rs/commit/fc6d34843a1b360df4cbc6bbecf263e6318b9179))


### Bug Fixes

* Activate must_use_candidate and Append must_use ([#197](https://github.com/MikuroXina/bms-rs/issues/197)) ([6ca4eb7](https://github.com/MikuroXina/bms-rs/commit/6ca4eb7963123e9c711faad7e97cf2ceb45c78a8))
* base eprintln and clippy(never_loop) ([#174](https://github.com/MikuroXina/bms-rs/issues/174)) ([ed9d9fc](https://github.com/MikuroXina/bms-rs/commit/ed9d9fcfc916ee1e906eb000a7921764cbb3a053))
* Bring back notes public ([#194](https://github.com/MikuroXina/bms-rs/issues/194)) ([b53edac](https://github.com/MikuroXina/bms-rs/commit/b53edacf5107e4471bdc1ad4e9138b08bbb9a1f7))
* Change Notes to hold data by logical channel ([#190](https://github.com/MikuroXina/bms-rs/issues/190)) ([be7d484](https://github.com/MikuroXina/bms-rs/commit/be7d4842a82271ce8d4d644d22940d2c27f20133))
* Change WavObj to hold channel id ([#195](https://github.com/MikuroXina/bms-rs/issues/195)) ([bd66208](https://github.com/MikuroXina/bms-rs/commit/bd66208839d744c4111d6572160f7358a895d0ea))
* impl channel key bindings with Bms ([#188](https://github.com/MikuroXina/bms-rs/issues/188)) ([3adab1d](https://github.com/MikuroXina/bms-rs/commit/3adab1d575950a8e73eeb55310eb6bf74ed97923))
* Minor pedantic fixes by cargo clippy ([#189](https://github.com/MikuroXina/bms-rs/issues/189)) ([1fa5c97](https://github.com/MikuroXina/bms-rs/commit/1fa5c976f9ba5a9ea6f2f3f86dc6b7af103984ee))
* Rename Obj into WavObj ([#192](https://github.com/MikuroXina/bms-rs/issues/192)) ([ec6eae8](https://github.com/MikuroXina/bms-rs/commit/ec6eae8f1cc5f4631bb635df9074ae8938228fd9))
* split `PlayerSide` for `ids_by_key` ([#168](https://github.com/MikuroXina/bms-rs/issues/168)) ([d9c73fc](https://github.com/MikuroXina/bms-rs/commit/d9c73fcf5f701fe8a406dd76dedd40eca3e63b7d))
* use "WithPos" postfix ([#152](https://github.com/MikuroXina/bms-rs/issues/152)) ([78a0c4a](https://github.com/MikuroXina/bms-rs/commit/78a0c4adbe1b9327e39d3876dfd0bd747d3ed2db))
* use struct-binding function for single step ([#154](https://github.com/MikuroXina/bms-rs/issues/154)) ([bad36e8](https://github.com/MikuroXina/bms-rs/commit/bad36e812ce2024c721d8ffbbabeddbdfa598319))


### Miscellaneous Chores

* Release 0.9.0 ([#191](https://github.com/MikuroXina/bms-rs/issues/191)) ([bc55df9](https://github.com/MikuroXina/bms-rs/commit/bc55df988c0ffaa8d784cb641178adcf20f3423b))

## [0.8.0](https://github.com/MikuroXina/bms-rs/compare/v0.7.0...v0.8.0) (2025-08-03)


### ⚠ BREAKING CHANGES

* Rearrange struct positions ([#143](https://github.com/MikuroXina/bms-rs/issues/143))
* add beatoraja & remaining tokens ([#141](https://github.com/MikuroXina/bms-rs/issues/141))
* use BigUInt & Decimal ([#139](https://github.com/MikuroXina/bms-rs/issues/139))
* Add note channel preset, modify `LexWarning` ([#135](https://github.com/MikuroXina/bms-rs/issues/135))
* add playing warning/error checking & classify lex/parse/playing warnings ([#134](https://github.com/MikuroXina/bms-rs/issues/134))
* Change usage of `parse` and `Bms::from_token_stream`, only return warnings ([#132](https://github.com/MikuroXina/bms-rs/issues/132))

### Features

* add beatoraja & remaining tokens ([#141](https://github.com/MikuroXina/bms-rs/issues/141)) ([4fbf6c6](https://github.com/MikuroXina/bms-rs/commit/4fbf6c6d7d79d33e06cd86f124d3ef9642970975))
* Add note channel preset, modify `LexWarning` ([#135](https://github.com/MikuroXina/bms-rs/issues/135)) ([6fcd830](https://github.com/MikuroXina/bms-rs/commit/6fcd8301de65407391e1cff66da069c64020701f))
* add playing warning/error checking & classify lex/parse/playing warnings ([#134](https://github.com/MikuroXina/bms-rs/issues/134)) ([7a9732d](https://github.com/MikuroXina/bms-rs/commit/7a9732d4702038d0c0b708e99dbe7f4593de41a6))
* **bmson:** add beatoraja ext ([#136](https://github.com/MikuroXina/bms-rs/issues/136)) ([d76b4df](https://github.com/MikuroXina/bms-rs/commit/d76b4dfaec954c54320ae127396927ade56056e9))
* Change usage of `parse` and `Bms::from_token_stream`, only return warnings ([#132](https://github.com/MikuroXina/bms-rs/issues/132)) ([94a8bb4](https://github.com/MikuroXina/bms-rs/commit/94a8bb4f87226a81313e8ca7e3272e651cdb3768))
* Use AST to support more situations, pass the insane test in BMS command memo ([#126](https://github.com/MikuroXina/bms-rs/issues/126)) ([2fdaa45](https://github.com/MikuroXina/bms-rs/commit/2fdaa452537750b41a6415b9c22d5ada2d16aafe))
* use BigUInt & Decimal ([#139](https://github.com/MikuroXina/bms-rs/issues/139)) ([4f17a7c](https://github.com/MikuroXina/bms-rs/commit/4f17a7c319ee17183ccf7f7eeb5d918c1feddeeb))


### Bug Fixes

* Fill files test, add support for file path with spaces ([#140](https://github.com/MikuroXina/bms-rs/issues/140)) ([f8eb0e1](https://github.com/MikuroXina/bms-rs/commit/f8eb0e18a66af0f0a8771b0d5693c045e68e2235))
* Rearrange struct positions ([#143](https://github.com/MikuroXina/bms-rs/issues/143)) ([d05edf4](https://github.com/MikuroXina/bms-rs/commit/d05edf471d9f92d500132618e19f1ff464e031ad))
* serde error & clippy warnings ([#130](https://github.com/MikuroXina/bms-rs/issues/130)) ([b62a742](https://github.com/MikuroXina/bms-rs/commit/b62a742954cf10dfbd76938647186a1042aee934))


### Miscellaneous Chores

* release 0.8.0 ([#124](https://github.com/MikuroXina/bms-rs/issues/124)) ([440c018](https://github.com/MikuroXina/bms-rs/commit/440c018e1465ec5833234d2e76f2890fa2682795))

## [0.7.0](https://github.com/MikuroXina/bms-rs/compare/v0.6.0...v0.7.0) (2025-06-22)


### ⚠ BREAKING CHANGES

* Add prompting interface ([#119](https://github.com/MikuroXina/bms-rs/issues/119))
* Bump edition to 2024 ([#113](https://github.com/MikuroXina/bms-rs/issues/113))

### Features

* Add prompting interface ([#119](https://github.com/MikuroXina/bms-rs/issues/119)) ([aa50d15](https://github.com/MikuroXina/bms-rs/commit/aa50d154ac8610c056830f882d62d4e37f513f86))
* Bump edition to 2024 ([#113](https://github.com/MikuroXina/bms-rs/issues/113)) ([3bd2d8f](https://github.com/MikuroXina/bms-rs/commit/3bd2d8f9dfcad2563151bf7e934c3a4ebd5256d2))


### Miscellaneous Chores

* release 0.7.0 ([#121](https://github.com/MikuroXina/bms-rs/issues/121)) ([4aa0b27](https://github.com/MikuroXina/bms-rs/commit/4aa0b2767281c56241d8691a8efe7355a0758b50))

## [0.6.0](https://github.com/MikuroXina/bms-rs/compare/v0.5.0...v0.6.0) (2025-01-14)


### Features

* **bms:** Deal with comment/non-command, tested ([#104](https://github.com/MikuroXina/bms-rs/issues/104)) ([6fe1f21](https://github.com/MikuroXina/bms-rs/commit/6fe1f21ba15592a5bc4746f5dbfd1d9cbcc11301))
* **bms:** Full random/switch support ([#109](https://github.com/MikuroXina/bms-rs/issues/109)) ([6357d56](https://github.com/MikuroXina/bms-rs/commit/6357d56ac1f40e5340bcd4be2d400595ecc15bc1))
* Derive some traits for RngMock ([#110](https://github.com/MikuroXina/bms-rs/issues/110)) ([15facc9](https://github.com/MikuroXina/bms-rs/commit/15facc995135619481caf0faf64cb6498385fa03))
* replace todo!() with returning Error ([#101](https://github.com/MikuroXina/bms-rs/issues/101)) ([76846d6](https://github.com/MikuroXina/bms-rs/commit/76846d68e25fb44d4fcdfdb1dd9e8863127f0c03))


### Bug Fixes

* **bms/lex:** use Cow&lt;'a, str&gt;, for performance? ([#99](https://github.com/MikuroXina/bms-rs/issues/99)) ([44fa2bd](https://github.com/MikuroXina/bms-rs/commit/44fa2bd08451bc9dd2ad38c0617d930f28dd341b))
* Fix Errors about Cursor when parsing source with no "\r\n" in the end. ([#106](https://github.com/MikuroXina/bms-rs/issues/106)) ([4b07f2d](https://github.com/MikuroXina/bms-rs/commit/4b07f2dae42676c7625de12cf84965a7b44ef1db))

## [0.5.0](https://github.com/MikuroXina/bms-rs/compare/v0.4.4...v0.5.0) (2025-01-06)


### ⚠ BREAKING CHANGES

* Support Bemuse extensions ([#93](https://github.com/MikuroXina/bms-rs/issues/93))

### Features

* Support base 62 object id ([#95](https://github.com/MikuroXina/bms-rs/issues/95)) ([31d753b](https://github.com/MikuroXina/bms-rs/commit/31d753b944a8ea5f1a97f854880f8c1a2f41ebbf))
* Support Bemuse extensions ([#93](https://github.com/MikuroXina/bms-rs/issues/93)) ([5d027eb](https://github.com/MikuroXina/bms-rs/commit/5d027ebd8e0274d9aab6a5c9a704bcce4d5f8aea))


### Miscellaneous Chores

* Release 0.5.0 ([#96](https://github.com/MikuroXina/bms-rs/issues/96)) ([322941c](https://github.com/MikuroXina/bms-rs/commit/322941c6ea89eb84517370df33540a953be2de90))

## [0.4.4](https://github.com/MikuroXina/bms-rs/compare/v0.4.3...v0.4.4) (2023-11-09)


### Bug Fixes

* Subtitle parse bug fix ([#50](https://github.com/MikuroXina/bms-rs/issues/50)) ([3617e87](https://github.com/MikuroXina/bms-rs/commit/3617e87efe4d86c25e5fb005856809b5911491aa))

## [0.4.3](https://github.com/MikuroXina/bms-rs/compare/v0.4.2...v0.4.3) (2023-10-08)


### Features

* Enforce Error types with thiserror ([#39](https://github.com/MikuroXina/bms-rs/issues/39)) ([01fb306](https://github.com/MikuroXina/bms-rs/commit/01fb306a8b463d99b35fc83cf83c7d1f5bf9bf35))


### Bug Fixes

* Fix non-standard resource name can't be handled correctly. ([#37](https://github.com/MikuroXina/bms-rs/issues/37)) ([446303d](https://github.com/MikuroXina/bms-rs/commit/446303d8d678a78acdc5cb4891ddee702891e2a9))


### Miscellaneous Chores

* Release 0.4.3 ([#40](https://github.com/MikuroXina/bms-rs/issues/40)) ([e26b9bb](https://github.com/MikuroXina/bms-rs/commit/e26b9bb2779de5449936e772d5f15e44b22c4c2e))

## [0.4.2](https://github.com/MikuroXina/bms-rs/compare/v0.4.1...v0.4.2) (2023-10-04)


### Bug Fixes

* Fix Bgm Obj can't be fully parsed ([#35](https://github.com/MikuroXina/bms-rs/issues/35)) ([ff481ce](https://github.com/MikuroXina/bms-rs/commit/ff481ce7a2e4efaa1018fba510871ef1a9a2e486))

## [0.4.1](https://github.com/MikuroXina/bms-rs/compare/v0.4.0...v0.4.1) (2023-10-03)


### Bug Fixes

* Fix to remove from ids_by_key ([#33](https://github.com/MikuroXina/bms-rs/issues/33)) ([c46abde](https://github.com/MikuroXina/bms-rs/commit/c46abde3a4f75d3a0148344c3ed3cc24db8ee36a))

## [0.4.0](https://github.com/MikuroXina/bms-rs/compare/v0.3.0...v0.4.0) (2023-10-03)


### ⚠ BREAKING CHANGES

* Store multiple notes by id ([#29](https://github.com/MikuroXina/bms-rs/issues/29))

### Features

* Store multiple notes by id ([#29](https://github.com/MikuroXina/bms-rs/issues/29)) ([ae6d531](https://github.com/MikuroXina/bms-rs/commit/ae6d531077a397367b282c060a3ddf7d818b26c2))


### Miscellaneous Chores

* Relase 0.4.0 ([#31](https://github.com/MikuroXina/bms-rs/issues/31)) ([5e6f2e0](https://github.com/MikuroXina/bms-rs/commit/5e6f2e075cf9e5fb859e9b5b60ee7a7ff911ce7a))

## 0.3.0 (2023-04-17)


### ⚠ BREAKING CHANGES

* Support bmson ([#12](https://github.com/MikuroXina/bms-rs/issues/12))
* Change to use Track time in SectionLenChangeObj ([#9](https://github.com/MikuroXina/bms-rs/issues/9))

### Features

* Add serde feature ([#3](https://github.com/MikuroXina/bms-rs/issues/3)) ([d8a2a8b](https://github.com/MikuroXina/bms-rs/commit/d8a2a8b540323ed23d4bb74cb1dc7dd804e01413))
* Setup ([99845d5](https://github.com/MikuroXina/bms-rs/commit/99845d5e0143781d38e1a153efd6d689c71c6c01))
* Support bmson ([#12](https://github.com/MikuroXina/bms-rs/issues/12)) ([fe08259](https://github.com/MikuroXina/bms-rs/commit/fe08259b9232ea491d1770346611bf43caed9cd9))


### Bug Fixes

* Change to use Track time in SectionLenChangeObj ([#9](https://github.com/MikuroXina/bms-rs/issues/9)) ([e321707](https://github.com/MikuroXina/bms-rs/commit/e321707dafd98a6af3c9ba4e4b196fea37452458))
* Fix to parse BACKBMP ([#7](https://github.com/MikuroXina/bms-rs/issues/7)) ([b23b67b](https://github.com/MikuroXina/bms-rs/commit/b23b67bdc98b2b8ac870247c75759e8542f76529))


### Miscellaneous Chores

* Release 0.3.0 ([#15](https://github.com/MikuroXina/bms-rs/issues/15)) ([661e278](https://github.com/MikuroXina/bms-rs/commit/661e278cc22d552ccdf70e79a9e40e391d9b32dd))
