# Change Log

## [0.4.0] - 2016-04-20
### Fixed
- Update for Rust v1.8

## [0.4.0] - 2015-06-27
### Fixed
- Transfer values in base64-encoding to save space in messages (MTU)

## [0.3.1] - 2015-06-26
### Fixed
- Major race condition bug in find_value/find_node

## [0.3.0] - 2015-06-19
### Changed
- Change network protocol (for IPs)
- Remove re-publishing (and thus Remove() and RemoveKey())

## [0.2.1] - 2015-06-08
### Changed
- Discard old values if a node publishes a new value

## [0.2.0] - 2015-06-04
### Fixed
- Split responses to individual packets to respect MTU

## [0.1.3] - 2015-06-03
### Fixed
- Just send Store message to nodes that are up
- Empty dbus errors caused panic!()
- Do not check remote IP addresses for tests

## [0.1.2] - 2015-06-02
### Fixed
- Better Dbus errors
### Changed
- Check remote IP addresses

## [0.1.1] - 2015-05-29
### Changed
- Better documentation
### Fixed
- Fix error in communication between IPv4 and IPv6

## [0.1.0] - 2015-05-26
### Added
- Initial Release


