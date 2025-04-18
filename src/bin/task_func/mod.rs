use defmt::info;
use embassy_stm32::gpio::Output;
use embassy_stm32::mode::Blocking;
use embassy_stm32::{peripherals, spi, Config};
use embassy_time::{Delay, Duration, Timer};
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::prelude::RgbColor;
use embedded_graphics::primitives::{Line, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle};
use embedded_hal_async::spi::SpiDevice;
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_nrf24l01::{Configuration, CrcMode, DataRate, NRF24L01};
use mipidsi::interface::SpiInterface;
use mipidsi::models::ILI9486Rgb666;
use mipidsi::options::Orientation;
use mipidsi::Builder;
use profont::PROFONT_24_POINT;
use embedded_graphics::text::Text;
use embedded_graphics::{prelude::*, pixelcolor::Rgb666};
use embassy_executor::{Executor, Spawner};
use core::sync::atomic::{AtomicBool, AtomicUsize,Ordering};
use crate::{MENU_STATE, RF_STATE};
use embedded_hal::digital::OutputPin;

#[embassy_executor::task]
pub async fn dislay_task(
    mut spi: spi::Spi<'static, Blocking>,
    ce:Output<'static>,
    dc:Output<'static>,
    rst: Output<'static>,
) {
    let mut buffer = [0_u8; 512];
    let spi_device = ExclusiveDevice::new_no_delay(spi, ce).unwrap();
    // let spi_device = ExclusiveDevice::new_no_delay(spi, cs_output).unwrap();
    let di = SpiInterface::new(spi_device, dc, &mut buffer);
    let mut delay = Delay;
    let s_style = MonoTextStyle::new(&PROFONT_24_POINT, Rgb666::WHITE);
    let sel_style = MonoTextStyle::new(&PROFONT_24_POINT, Rgb666::BLUE);
    let r_style = PrimitiveStyleBuilder::new()
        .fill_color(Rgb666::BLACK) 
        .build();
    let menu_list = ["1 MENU", "2 MENU", "3 MENU"];
    let mut flag= 0_usize;
    let mut rf_flag= false;
    let mut display = Builder::new(ILI9486Rgb666, di)
    .reset_pin(rst)
    .init(&mut delay).unwrap();
    
    display.clear(Rgb666::BLACK).unwrap();
    display.set_orientation(Orientation::default().flip_horizontal()).unwrap();
    
    // display.set_orientation(Orientation::Portrait(true)).unwrap();
    loop{
        if flag!=MENU_STATE.load(Ordering::Acquire){
            Rectangle::new(Point::new(1, 10), Size::new(30, 130))
                .into_styled(r_style)
                .draw(&mut display)
                .unwrap();
            Rectangle::new(Point::new(20, 180), Size::new(200, 50))
                .into_styled(r_style)
                .draw(&mut display)
                .unwrap();
            Text::new("->", Point::new(1, (MENU_STATE.load(Ordering::Acquire) as i32 * 30)+15), sel_style)
                .draw(&mut display)
                .unwrap();
            flag=MENU_STATE.load(Ordering::Acquire);
        };
        menu_list.iter().enumerate().for_each(|(len, str)| {
            let posion = len + 1;
            if flag==posion{
                let description=match flag {
                    1=>"SOMEEEEE",
                    2=>"TWOOOOOOOO",
                    3=>"THREEEE",
                    _=>"UNKOWN"
                };
                Text::new(str, Point::new(40, (posion as i32 * 30)+15), sel_style)
                    .draw(&mut display)
                    .unwrap();
                Line::new(Point::new(1, 150), Point::new(310, 150))
                    .into_styled(PrimitiveStyle::with_stroke(Rgb666::WHITE, 1))
                    .draw(&mut display).unwrap();
                Text::new(description, Point::new(20, 200), s_style)
                    .draw(&mut display)
                    .unwrap();
            }else {
                Text::new(str, Point::new(40, (posion as i32 * 30)+15), s_style)
                    .draw(&mut display)
                    .unwrap();
            }
            // RF_STATE
        });
        Line::new(Point::new(1, 300), Point::new(310, 300))
            .into_styled(PrimitiveStyle::with_stroke(Rgb666::WHITE, 1))
            .draw(&mut display).unwrap();
        Text::new("RF-DEVICE:", Point::new(20, 340), s_style)
            .draw(&mut display)
            .unwrap();
        
        
        match rf_flag {
            true=>{
                Text::new("ENABLE", Point::new(200, 340), s_style)
                    .draw(&mut display)
                    .unwrap();
            },
            false=>{
                Text::new("DISABLE", Point::new(200, 340), s_style)
                    .draw(&mut display)
                    .unwrap();
            }
        }
        Line::new(Point::new(1, 360), Point::new(310, 360))
            .into_styled(PrimitiveStyle::with_stroke(Rgb666::WHITE, 1))
            .draw(&mut display).unwrap();
        if rf_flag!=RF_STATE.load(Ordering::Acquire){
            Rectangle::new(Point::new(200, 315), Size::new(300, 25))
                .into_styled(r_style)
                .draw(&mut display)
                .unwrap();
            rf_flag=RF_STATE.load(Ordering::Acquire)
        }
        
        // Text::new("->", Point::new(1, 35), sel_style)
        //     .draw(&mut display)
        //     .unwrap();

        // info!("TRUE FALSE {:?}",rf_flag);
        Timer::after(Duration::from_nanos(1)).await;
        // Timer::after_ticks(10000).await;
        // info!("SPISPI");
    }
    // for n in 0u32.. {
    //     let mut write: String<128> = String::new();
    //     // core::write!(&mut write, "Hello DMA World {}!\r\n", n).unwrap();
    //     unsafe {
    //         let result = spi.blocking_transfer_in_place(write.as_bytes_mut());
    //         if let Err(_) = result {
    //             defmt::panic!("crap");
    //         }
    //     }
        
    // }
}

#[embassy_executor::task]
pub async fn rf_rec(
    spi: spi::Spi<'static, Blocking>,
    rf_ce: Output<'static>,
    rf_csn: Output<'static>,
) {
    Timer::after(Duration::from_millis(100)).await;
    // let mut buffer = [0_u8; 512];
    // let spi_device = ExclusiveDevice::new_no_delay(spi, rf_ce).unwrap();
    // let di = SpiInterface::new(spi_device, rf_ce, &mut buffer);
    let mut nrf24 = NRF24L01::new(rf_ce,rf_csn, spi).unwrap();
    nrf24.set_frequency(8).unwrap();
    nrf24.set_auto_retransmit(15, 15).unwrap();
    nrf24.set_rf(&DataRate::R2Mbps, 0).unwrap();
    nrf24
        .set_pipes_rx_enable(&[true, false, false, false, false, false])
        .unwrap();
    nrf24
        .set_auto_ack(&[true, false, false, false, false, false])
        .unwrap();
    nrf24.set_pipes_rx_lengths(&[None; 6]).unwrap();
    nrf24.set_crc(CrcMode::TwoBytes).unwrap();
    nrf24.set_rx_addr(0, &b"fnord"[..]).unwrap();
    nrf24.set_tx_addr(&b"fnord"[..]).unwrap();
    nrf24.flush_rx().unwrap();
    nrf24.flush_tx().unwrap();
    let mut rx = nrf24.rx().unwrap();
    loop {
        if rx.device().is_connected().unwrap(){
            // info!("DEVICE CONNECTED");
            RF_STATE.store(true, Ordering::Release);
            
        }
        if let Some(pipe) = rx.can_read().unwrap() {
            // iprintln!(stim, "Reading from pipe: {}", pipe);
            let payload = rx.read();
            match payload {
                Ok(p) => {
                    info!("Payload received {:?}", p.as_ref());
                //    iprintln!(stim, "Payload received: {:?}", p.as_ref());
                    // leds[Direction::West].on();
                }
                Err(_) => {
                    info!("Could not read payload");
                    // iprintln!(stim, "Could not read payload");
                    // leds[Direction::North].on();
                }
            }
        }
        Timer::after(Duration::from_millis(10)).await;
    }
}


