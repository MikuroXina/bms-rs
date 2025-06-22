# Changelog

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
