#![no_std]
#![no_main]

mod task_func;
use core::fmt::Write;
use core::str::from_utf8;
use cortex_m_rt::entry;
use defmt::*;
use embassy_executor::{Executor, Spawner};
use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};
use embassy_stm32::time::mhz;
use embassy_stm32::{spi, Config};
use embassy_time::{Delay, Instant, Timer};

use static_cell::StaticCell;
use task_func::dislay_task;
use {defmt_rtt as _, panic_probe as _};






static EXECUTOR: StaticCell<Executor> = StaticCell::new();
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Hello World!");

    let mut config = Config::default();
    {
        use embassy_stm32::rcc::*;

        config.rcc.hsi = Some(HSIPrescaler::DIV1); // HSI = 64 MHz
        config.rcc.csi = true;
        config.rcc.pll1 = Some(Pll {
            source: PllSource::HSI,           // PLL 입력 = HSI (64 MHz)
            prediv: PllPreDiv::DIV4,          // VCO 입력 = 64 / 4 = 16 MHz
            mul: PllMul::MUL50,               // VCO 출력 = 16 * 50 = 800 MHz
            divp: Some(PllDiv::DIV2),         // PLL1_P = 800 / 2 = 400 MHz (SYSCLK)
            divq: Some(PllDiv::DIV8),         // PLL1_Q = 800 / 8 = 100 MHz (예: SPI3 등)
            divr: None,
        });
        config.rcc.sys = Sysclk::PLL1_P;      // SYSCLK = 400 MHz
        config.rcc.ahb_pre = AHBPrescaler::DIV2;   // AHB = 400 / 2 = 200 MHz
        config.rcc.apb1_pre = APBPrescaler::DIV2;  // APB1 = 200 / 2 = 100 MHz
        config.rcc.apb2_pre = APBPrescaler::DIV2;  // APB2 = 200 / 2 = 100 MHz
        config.rcc.apb3_pre = APBPrescaler::DIV2;  // APB3 = 200 / 2 = 100 MHz
        config.rcc.apb4_pre = APBPrescaler::DIV2;  // APB4 = 200 / 2 = 100 MHz

        config.rcc.voltage_scale = VoltageScale::Scale1; // 최대 클럭 지원 (400 MHz 이상)
    }
    let p: embassy_stm32::Peripherals = embassy_stm32::init(config);
    // let ce = Input::new(p.PA4, Pull::Up);
    
    // let cs = gpioa.pa4.into_push_pull_output();
    let mut spi_config = spi::Config::default();
    spi_config.frequency = mhz(20);
    //ILI9486 SPI DISPLAY SETTING
    let back_l = Output::new(p.PA3, Level::High, Speed::High);
    let ce = Output::new(p.PA4, Level::High, Speed::Low);
    let dc = Output::new(p.PA2, Level::High, Speed::Low);
    let mosi = p.PA7;
    let miso = p.PA6;
    let sck = p.PA5;
    let rst: Output = Output::new(p.PA1, Level::High, Speed::Low);
    // let s_rst = gpioa.pa1.into_push_pull_output();
    let spi = spi::Spi::new_blocking(p.SPI1, sck, mosi, miso, spi_config);
    // let spi = spi::Spi::new();
    
    // let executor = EXECUTOR.init(Executor::new());
    spawner.spawn(dislay_task(spi,ce,dc,rst)).ok();
    spawner.spawn(run_med()).ok();
    loop{
        info!("MAIN");
        Timer::after_ticks(10000).await;
    }
    // loop{
    //     info!("read via asdasdasdasd");
    // }
    // executor.run(|spawner| {
    //     unwrap!(spawner.spawn(main_task(spi)));
    // })
}


#[embassy_executor::task]
async fn run_med() {
    loop {
        info!("MID");
        Timer::after_ticks(10000).await;
    }
}
