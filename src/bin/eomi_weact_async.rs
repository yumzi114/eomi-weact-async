#![no_std]
#![no_main]

mod task_func;
use core::{alloc, mem};
use core::fmt::Write;
use core::str::{from_utf8, Bytes};
use cortex_m_rt::entry;
use defmt::*;
use embassy_executor::{Executor, Spawner};
use embassy_stm32::dma::NoDma;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::flash::Blocking;
use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};
use embassy_stm32::pac::Interrupt::{DMA1_STREAM0,DMA1_STREAM1};
use embassy_stm32::sdmmc::Sdmmc;
use embassy_stm32::time::mhz;
use embassy_stm32::usart::{self, BufferedInterruptHandler, Config as USARTConfig, RxDma, TxDma, Uart, UartTx };
use embassy_stm32::{bind_interrupts, pac, peripherals, sdmmc, spi, Config};
use embassy_sync::blocking_mutex::raw::{NoopRawMutex, ThreadModeRawMutex};
use embassy_time::{Delay, Duration, Instant, Timer};

use embedded_nrf24l01::NRF24L01;
use heapless::{String, Vec};
use static_cell::StaticCell;
use task_func::{dislay_task, rf_rec, usart_reader};
use {defmt_rtt as _, panic_probe as _};
use embedded_hal_1::digital::OutputPin;
use embedded_hal_1::digital::ErrorType;
use core::sync::atomic::{AtomicBool, AtomicUsize,Ordering};
// static CHANNEL: StaticCell<Channel<ThreadModeRawMutex, u32, 4>> = StaticCell::new();
type MyChannel = Channel<ThreadModeRawMutex, Vec<u8, 16>, 4>;
static MENU_STATE: AtomicUsize = AtomicUsize::new(1_usize);
static RF_STATE: AtomicBool = AtomicBool::new(false);
static EXECUTOR: StaticCell<Executor> = StaticCell::new();
// static MSG: Q16<String<32>> = Q16::new();
static MSG: Q16<Vec<u8, 16>> = Q16::new();
static CHANNEL: StaticCell<MyChannel> = StaticCell::new();

use embassy_sync::mutex::Mutex;
use embassy_sync::channel::Channel;
use core::cell::RefCell;
use heapless::mpmc::Q2;
use heapless::mpmc::Q16;

bind_interrupts!(struct Irqs {
    SDMMC1 => sdmmc::InterruptHandler<peripherals::SDMMC1>;
    // USART1 => usart::InterruptHandler<peripherals::USART1>;
});
bind_interrupts!(struct Birqs {
    USART1 => BufferedInterruptHandler<embassy_stm32::peripherals::USART1>;
});
// static MESSAGE_CHANNEL: Channel<ThreadModeRawMutex, heapless::String<32>, 1> = Channel::new();
// static MESSAGE_Q: Q2<String<32>, 2, NoopRawMutex> = Q2::new();
// use embedded_hal::digital::v2::OutputPin;
use core::convert::Infallible;

// pub struct SafeOutputPin<P>(pub P);

// impl<P: OutputPin<Error = Infallible>> ErrorType for SafeOutputPin<P> {
//     type Error = Infallible;
// }

// impl<P: OutputPin<Error = Infallible>> OutputPin for SafeOutputPin<P> {
//     fn set_high(&mut self) -> Result<(), Self::Error> {
//         self.0.set_high()
//     }

//     fn set_low(&mut self) -> Result<(), Self::Error> {
//         self.0.set_low()
//     }
// }

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
            divq: Some(PllDiv::DIV4),         // default clock chosen by SDMMCSEL. 200 Mhz
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
    // let rf_ce: SafeOutputPin<peripherals::PA8> = SafeOutputPin(p.PA8);
    // let rf_csn: SafeOutputPin<peripherals::PA9> = SafeOutputPin(p.PA9);
    let up_button = Input::new(p.PA11,Pull::Down);
    let down_button = Input::new(p.PA12,Pull::Down);
    let mut blue_led=Output::new(p.PE3, Level::Low, Speed::Low);
    // blue_led.
    // let cs = gpioa.pa4.into_push_pull_output();
    let mut spi_config = spi::Config::default();
    spi_config.frequency = mhz(60);
    //ILI9486 SPI DISPLAY SETTING
    let back_l = Output::new(p.PA3, Level::High, Speed::High);
    let ce = Output::new(p.PA4, Level::High, Speed::Low);
    let dc = Output::new(p.PA2, Level::High, Speed::Low);
    let mosi = p.PA7;
    let miso = p.PA6;
    let sck = p.PA5;
    let rst: Output = Output::new(p.PA1, Level::High, Speed::Low);
    // let s_rst = gpioa.pa1.into_push_pull_output();
    let spi= spi::Spi::new_blocking(p.SPI1, sck, mosi, miso, spi_config);
    // let spi = spi::Spi::new();
    let mut rf_spi_config = spi::Config::default();
    rf_spi_config.frequency = mhz(1);
    let rf_ce =Output::new(p.PD9, Level::High, Speed::Low);
    let rf_csn = Output::new(p.PB12, Level::High, Speed::Low);
    let rf_sck = p.PB13;
    let rf_mosi = p.PB15;
    let rf_miso = p.PB14;
    let rf_spi= spi::Spi::new_blocking(p.SPI2, rf_sck, rf_mosi, rf_miso, rf_spi_config);
    let mut buffer = [0_u8; 512];
    // let mut nrf24 = NRF24L01::new(rf_ce,rf_csn, rf_spi).unwrap();
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
    let usart_config: USARTConfig = USARTConfig::default();
    // let asd = DMA1_STREAM0;
    // let tx_dma = RxDma::DMA1_STREAM0; // 이게 스트림 1
    // let rx_dma = DMA::new_stream0(); 
    
    let usart: peripherals::USART1 = p.USART1;
    let rx: peripherals::PB7 =p.PB7;
    let tx: peripherals::PB6=p.PB6;
    let tx_dma =p.DMA1_CH1;
    let rx_dma=p.DMA1_CH0;
    

    // let mut channel: Channel<NoopRawMutex, u32, 3> = Channel::<NoopRawMutex, u32, 3>::new();
    // let mut usart = Uart::new(p.USART1, p.PB7, p.PB6, Irqs, p.DMA1_CH5, p.DMA1_CH1, usart_config).unwrap();
    // let uart_demo_rx = UartRx::new(
    //     p.UART7,
    //     Irqs,
    //     p.PE7,
    //     p.DMA1_CH1,
    //     embassy_stm32::usart::Config::default(),
    // ).unwrap();
    // let mut usart= Uart::new_blocking(p.UART7, p.PE7, p.PE8, usart_config).unwrap();
    // unwrap!(usart.blocking_write(b"Type 8 chars to echo!\r\n"));
    // let (mut tx, rx) = usart.split();
    info!("Configured clock: {}", sdmmc.clock().0);
    // let card = unwrap!(sdmmc.card());
    // info!("Card: {:#?}", Debug2Format(card));
    // unwrap!(sdmmc.init_card(mhz(10)).await);
    // let card = unwrap!(sdmmc.card());

    // info!("Card: {:#?}", Debug2Format(card));
    // let executor = EXECUTOR.init(Executor::new());
    let channel: Channel<ThreadModeRawMutex, Vec<u8, 16>, 4> = Channel::<ThreadModeRawMutex, Vec<u8, 16>, 4>::new();
    let m_channel: &'static MyChannel=CHANNEL.init(channel);
    spawner.spawn(rf_rec(rf_spi,rf_ce,rf_csn)).ok();
    spawner.spawn(usart_reader(usart,rx,tx,usart_config,m_channel)).ok();
    
    spawner.spawn(dislay_task(spi,ce,dc,rst,m_channel)).ok();
    
    
    
    
    loop{
        //button interrupts
        //down
        // if let Err(e) = rf_task {
        //     RF_STATE.store(false, Ordering::Release);
        // }
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
        Timer::after(Duration::from_millis(10)).await;
    }
    // loop{
    //     info!("read via asdasdasdasd");
    // }
    // executor.run(|spawner| {
    //     unwrap!(spawner.spawn(main_task(spi)));
    // })
}


