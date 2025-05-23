use core::fmt::Debug;
use embedded_hal::{
    digital::OutputPin,
    spi::{Error, ErrorKind, ErrorType, Operation, SpiBus, SpiDevice},
};

#[allow(dead_code)]
#[derive(Debug)]
pub enum SpiDeviceWrapperError<E> {
    SpiBus(E),  // Errors wrapper from the SpiBus
    Lock,       // Error when attempting to lock the bus
    ChipSelect, // Error when interacting with the chip select gpio
}

// Allow to map the custom error types to error compatible with the SpiDevice trait
impl<E> Error for SpiDeviceWrapperError<E>
where
    E: Error + Debug,
{
    #[inline]
    fn kind(&self) -> ErrorKind {
        match self {
            SpiDeviceWrapperError::SpiBus(e) => e.kind(), // Fwd SpiBus error by converting them into ErroKind
            SpiDeviceWrapperError::Lock => ErrorKind::Other,
            SpiDeviceWrapperError::ChipSelect => ErrorKind::ChipSelectFault,
        }
    }
}

#[allow(dead_code)]
pub struct SpiDeviceWrapper<'a, S, P> {
    spi_bus: &'a mut S,
    ns_per_tick: u32,
    cs: P,
}

// ErrorType trait implementation for the SpiDeviceWrapper.
// This binds the custom error type to the wrapper, and since
// the type implements Error, it can be used as type Error.
impl<S, E, P> ErrorType for SpiDeviceWrapper<'_, S, P>
where
    S: SpiBus<u8, Error = E>,
    E: Error,
    P: OutputPin,
{
    type Error = SpiDeviceWrapperError<E>;
}

// Wrapper specific implementation
impl<'a, S, E, P> SpiDeviceWrapper<'a, S, P>
where
    S: SpiBus<u8, Error = E>,
    E: Error,
    P: OutputPin,
{
    pub fn new(spi_bus: &'a mut S, cs: P, ns_per_tick: u32) -> Self {
        SpiDeviceWrapper {
            spi_bus,
            ns_per_tick,
            cs,
        }
    }

    #[allow(dead_code)]
    #[inline]
    fn assert_cs(&mut self) -> Result<(), SpiDeviceWrapperError<E>> {
        self.cs
            .set_low()
            .map_err(|_| SpiDeviceWrapperError::ChipSelect)
    }

    #[allow(dead_code)]
    #[inline]
    fn deassert_cs(&mut self) -> Result<(), SpiDeviceWrapperError<E>> {
        self.cs
            .set_high()
            .map_err(|_| SpiDeviceWrapperError::ChipSelect)
    }
}

// SpiDevice trait implementation
impl<S, E, P> SpiDevice for SpiDeviceWrapper<'_, S, P>
where
    S: SpiBus<u8, Error = E>,
    E: Error,
    P: OutputPin,
{
    fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> Result<(), Self::Error> {
        // Locks the bus
        // TODO
        // Asserts the CS (Chip Select) pin.
        self.assert_cs()?;
        // Performs all the operations.
        for operation in operations {
            match operation {
                Operation::Read(words) => {
                    self.spi_bus
                        .read(words)
                        .map_err(SpiDeviceWrapperError::SpiBus)?;
                }
                Operation::Write(words) => {
                    self.spi_bus
                        .write(words)
                        .map_err(SpiDeviceWrapperError::SpiBus)?;
                }
                Operation::Transfer(in_buff, out_buff) => {
                    self.spi_bus
                        .transfer(in_buff, out_buff)
                        .map_err(SpiDeviceWrapperError::SpiBus)?;
                }
                Operation::TransferInPlace(words) => {
                    self.spi_bus
                        .transfer_in_place(words)
                        .map_err(SpiDeviceWrapperError::SpiBus)?;
                }
                Operation::DelayNs(delay_ns) => {
                    let mut delay_in_ticks = *delay_ns / self.ns_per_tick;
                    while delay_in_ticks > 0 {
                        delay_in_ticks -= 1;
                    }
                }
            }
        }
        // Flushes the bus.
        self.spi_bus
            .flush()
            .map_err(SpiDeviceWrapperError::SpiBus)?;
        // Deasserts the CS pin.
        self.deassert_cs()?;

        // Unlocks the bus.
        Ok(())
    }
}
