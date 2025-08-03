# Changelog

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
