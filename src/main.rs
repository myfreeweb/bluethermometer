#![no_std]
#![no_main]

use {
    byteorder::{BigEndian, ByteOrder, LittleEndian},
    fixed::types::{I30F2, I8F8},
    nrf51_hal::{
        hi_res_timer::{As16BitTimer, HiResTimer, TimerCc, TimerFrequency},
        temp::Temp,
    },
    panic_semihosting as _,
    rubble::{
        beacon::Beacon,
        link::{
            ad_structure::{AdStructure, ServiceUuids},
            AddressKind, DeviceAddress, MIN_PDU_BUF,
        },
        uuid::Uuid16,
    },
    rubble_nrf5x::radio::{BleRadio, PacketBuffer},
};

fn advertise_beacon(radio: &mut BleRadio, hw_addr: &[u8; 6], addr_kind: AddressKind, sensor_idx: u16, temperature: I8F8) {
    let mut addr = *hw_addr;
    BigEndian::write_u16(&mut addr[3..], sensor_idx);
    let device_address = DeviceAddress::new(addr, addr_kind);

    // rubble consumes ownership of the radio, can't use it anymore
    // rubble sets 4, for now
    // let tx_dbm = ctx.device.RADIO.txpower.read().txpower().bits();
    let tx_dbm = 4;

    let mut uid_frame = [
        0, tx_dbm, 0xDE, 0xAD, 0xBE, 0xEF, addr[0], addr[1], addr[2], addr[3], addr[4], addr[5], 0, 0xC0, 0xFF, 0xEE, 0, 0, 0, 0,
    ];
    BigEndian::write_u16(&mut uid_frame[16..], sensor_idx);

    Beacon::new(
        device_address,
        &[
            AdStructure::ServiceUuids16(ServiceUuids::from_uuids(true, &[Uuid16(0xFEAA)])),
            AdStructure::ServiceData16 {
                uuid: 0xFEAA,
                data: &uid_frame,
            },
        ],
    )
    .unwrap()
    .broadcast(radio);

    let mut tlm_frame = [0x20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    BigEndian::write_i16(&mut tlm_frame[4..], temperature.to_bits());

    Beacon::new(
        device_address,
        &[
            AdStructure::ServiceUuids16(ServiceUuids::from_uuids(true, &[Uuid16(0xFEAA)])),
            AdStructure::ServiceData16 {
                uuid: 0xFEAA,
                data: &tlm_frame,
            },
        ],
    )
    .unwrap()
    .broadcast(radio);
}

#[rtfm::app(device = nrf51, peripherals = true)]
const APP: () = {
    struct Resources {
        #[init([0; MIN_PDU_BUF])]
        ble_tx_buf: PacketBuffer,
        #[init([0; MIN_PDU_BUF])]
        ble_rx_buf: PacketBuffer,
        radio: BleRadio,
        devaddr: [u8; 6],
        devaddr_type: AddressKind,
        beacon_timer: HiResTimer<nrf51::TIMER1, u16>,
        onboard_temp: Temp,
    }

    #[init(resources = [ble_tx_buf, ble_rx_buf])]
    fn init(ctx: init::Context) -> init::LateResources {
        {
            // On reset the internal high frequency clock is used, but starting the HFCLK task
            // switches to the external crystal; this is needed for Bluetooth to work.

            ctx.device.CLOCK.tasks_hfclkstart.write(|w| unsafe { w.bits(1) });
            while ctx.device.CLOCK.events_hfclkstarted.read().bits() == 0 {}
        }

        let mut devaddr = [0u8; 6];
        let devaddr_lo = ctx.device.FICR.deviceaddr[0].read().bits();
        let devaddr_hi = ctx.device.FICR.deviceaddr[1].read().bits() as u16;
        LittleEndian::write_u32(&mut devaddr, devaddr_lo);
        LittleEndian::write_u16(&mut devaddr[4..], devaddr_hi);

        let addr_type_nrf = ctx.device.FICR.deviceaddrtype.read().deviceaddrtype();
        let devaddr_type = if addr_type_nrf.is_public() {
            AddressKind::Public
        } else {
            AddressKind::Random
        };

        let radio = BleRadio::new(ctx.device.RADIO, &ctx.device.FICR, ctx.resources.ble_tx_buf, ctx.resources.ble_rx_buf);

        let mut beacon_timer = ctx.device.TIMER1.as_16bit_timer();
        beacon_timer.set_frequency(TimerFrequency::Freq31250Hz);
        beacon_timer.set_compare_register(TimerCc::CC0, 31_250 * 2); // NOTE: 16-bit
        beacon_timer.enable_compare_interrupt(TimerCc::CC0);
        beacon_timer.clear();
        beacon_timer.start();

        init::LateResources {
            radio,
            devaddr,
            devaddr_type,
            onboard_temp: Temp::new(ctx.device.TEMP),
            beacon_timer,
        }
    }

    #[task(binds = TIMER1, resources = [beacon_timer, devaddr, devaddr_type, radio, onboard_temp])]
    fn timer1(ctx: timer1::Context) {
        // Acknowledge event so that the interrupt doesn't keep firing
        ctx.resources.beacon_timer.clear_compare_event(TimerCc::CC0);

        let addr = &*ctx.resources.devaddr;
        let addr_type = *ctx.resources.devaddr_type;
        let onb = I30F2::from_bits(ctx.resources.onboard_temp.measure().into_bits()); // fixed > fpa
        advertise_beacon(&mut *ctx.resources.radio, addr, addr_type, u16::max_value(), I8F8::from_num(onb));
        // TODO: one-wire
    }
};
