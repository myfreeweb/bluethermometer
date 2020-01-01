# bluethermometer

Pure Rust (using [Rubble] Bluetooth LE stack) firmware for the Nordic nRF51 series (nRF51822)
that exposes temperature sensors as [Eddystone] beacons (with temperature in telemetry frames).
Which is supported by [Home Assistant] among other things.

(for now, only onboard sensor. coming: one-wire)

[Rubble]: https://github.com/jonas-schievink/rubble
[Eddystone]: https://github.com/google/eddystone
[Home Assistant]: https://www.home-assistant.io/integrations/eddystone_temperature/

## License

This is free and unencumbered software released into the public domain.  
For more information, please refer to the `UNLICENSE` file or [unlicense.org](http://unlicense.org).

(Note: different licenses apply to dependencies.)
