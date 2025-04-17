use defmt::info;
use embassy_stm32::gpio::Output;
use embassy_stm32::mode::Blocking;
use embassy_stm32::{spi, Config};
use embassy_time::{Delay, Timer};
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::prelude::RgbColor;
use embedded_hal_async::spi::SpiDevice;
use embedded_hal_bus::spi::ExclusiveDevice;
use mipidsi::interface::SpiInterface;
use mipidsi::models::ILI9486Rgb666;
use mipidsi::options::Orientation;
use mipidsi::Builder;
use profont::PROFONT_24_POINT;
use embedded_graphics::text::Text;
use embedded_graphics::{prelude::*, pixelcolor::Rgb666};



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
    let sel_style = MonoTextStyle::new(&PROFONT_24_POINT, Rgb666::BLUE);
    let mut display = Builder::new(ILI9486Rgb666, di)
    .reset_pin(rst)
    .init(&mut delay).unwrap();
    
    display.clear(Rgb666::BLACK).unwrap();
    display.set_orientation(Orientation::default().flip_horizontal()).unwrap();
    
    // display.set_orientation(Orientation::Portrait(true)).unwrap();
    loop{
        Text::new("->", Point::new(30, 30), sel_style)
            .draw(&mut display)
            .unwrap();
        Timer::after_ticks(10000).await;
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
