#![no_std]
#![no_main]

mod task_func;
use core::fmt::Write;
use core::str::from_utf8;
use cortex_m_rt::entry;
use defmt::*;
use embassy_executor::{Executor, Spawner};
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};
use embassy_stm32::sdmmc::Sdmmc;
use embassy_stm32::time::{mhz, Hertz};
use embassy_stm32::{bind_interrupts, peripherals, sdmmc, spi, Config};
use embassy_time::{Delay, Duration, Instant, Timer};
use static_cell::StaticCell;
use task_func::dislay_task;
use {defmt_rtt as _, panic_probe as _};


use core::sync::atomic::{AtomicBool, AtomicUsize,Ordering};

static MENU_STATE: AtomicUsize = AtomicUsize::new(1_usize);
static EXECUTOR: StaticCell<Executor> = StaticCell::new();
bind_interrupts!(struct Irqs {
    SDMMC1 => sdmmc::InterruptHandler<peripherals::SDMMC1>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Hello World!");

    let mut config = Config::default();
    {
        use embassy_stm32::rcc::*;
        config.rcc.hse = Some(Hse {
            freq: Hertz(25_000_000),  // 25 MHz 외부 클럭
            mode: HseMode::Bypass,   // 크리스탈 모드로 설정
        });
        config.rcc.hsi = Some(HSIPrescaler::DIV1); // HSI = 64 MHz
        config.rcc.csi = true;
        config.rcc.pll1 = Some(Pll {
            source: PllSource::HSE,           // PLL 입력 = HSI (64 MHz)
            prediv: PllPreDiv::DIV4,          // VCO 입력 = 64 / 4 = 16 MHz
            mul: PllMul::MUL4,               // VCO 출력 = 16 * 50 = 800 MHz
            divp: Some(PllDiv::DIV2),         // PLL1_P = 800 / 2 = 400 MHz (SYSCLK)
            divq: Some(PllDiv::DIV4),         // default clock chosen by SDMMCSEL. 200 Mhz
            divr: None,
        });
        config.rcc.sys = Sysclk::PLL1_P;      // SYSCLK = 400 MHz
        config.rcc.ahb_pre = AHBPrescaler::DIV1;   // AHB = 400 / 2 = 200 MHz
        config.rcc.apb1_pre = APBPrescaler::DIV1;  // APB1 = 200 / 2 = 100 MHz
        config.rcc.apb2_pre = APBPrescaler::DIV1;  // APB2 = 200 / 2 = 100 MHz
        config.rcc.apb3_pre = APBPrescaler::DIV1;  // APB3 = 200 / 2 = 100 MHz
        config.rcc.apb4_pre = APBPrescaler::DIV1;  // APB4 = 200 / 2 = 100 MHz

        config.rcc.voltage_scale = VoltageScale::Scale1; // 최대 클럭 지원 (400 MHz 이상)
    }
    let p: embassy_stm32::Peripherals = embassy_stm32::init(config);
    // let ce = Input::new(p.PA4, Pull::Up);
    
    // let mut up_button = ExtiInput::new(p.PA11, p.EXTI11, Pull::Down);
    // let mut down_button = ExtiInput::new(p.PA10, p.EXTI10, Pull::Down);
    let up_button = Input::new(p.PA11,Pull::Down);
    let down_button = Input::new(p.PA10,Pull::Down);
    let mut blue_led=Output::new(p.PE3, Level::Low, Speed::Low);
    // blue_led.
    // let cs = gpioa.pa4.into_push_pull_output();
    let mut spi_config = spi::Config::default();
    spi_config.frequency = mhz(10);
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
    let mut sdmmc = Sdmmc::new_4bit(
        p.SDMMC1,
        Irqs,
        p.PC12,
        p.PD2,

        p.PC8,
        p.PC9,
        p.PC10,
        p.PC11,
        Default::default(),
    );
    info!("Configured clock: {}", sdmmc.clock().0);
    // let card = unwrap!(sdmmc.card());
    // info!("Card: {:#?}", Debug2Format(card));
    unwrap!(sdmmc.init_card(mhz(20)).await);
    let card = unwrap!(sdmmc.card());

    // info!("Card: {:#?}", Debug2Format(card));
    // let executor = EXECUTOR.init(Executor::new());
    spawner.spawn(dislay_task(spi,ce,dc,rst)).ok();
    spawner.spawn(run_med()).ok();
    loop{
        //button interrupts
        //down
        if up_button.is_high() {
            blue_led.set_high();
            if MENU_STATE.load(Ordering::Acquire)<3{
                let num = MENU_STATE.load(Ordering::Acquire);
                MENU_STATE.store(num+1, Ordering::Release);
                
            }else{
                MENU_STATE.store(1, Ordering::Release);
            }
            while up_button.is_high() {
                Timer::after(Duration::from_millis(10)).await;
            }
            blue_led.set_low();
        }

        if down_button.is_high() {
            blue_led.set_high();
            if MENU_STATE.load(Ordering::Acquire)>1{
                let num = MENU_STATE.load(Ordering::Acquire);
                MENU_STATE.store(num-1, Ordering::Release);
            }
            else{
                MENU_STATE.store(3, Ordering::Release);
            }
            // defmt::info!("DOWN pressed");
            while down_button.is_high() {
                Timer::after(Duration::from_millis(10)).await;
            }
            blue_led.set_low();
        }
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
