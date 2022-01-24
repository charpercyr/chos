use core::fmt;
use core::hint::spin_loop;
use core::intrinsics::transmute;
use core::marker::PhantomData;
use core::mem::{replace, MaybeUninit};

use modular_bitfield::specifiers::*;
use modular_bitfield::{bitfield, BitfieldSpecifier};

use super::port::{Port, PortData};
use crate::{ReadOnly, WriteOnly};

pub struct Uninit(());
pub struct Init(());

macro port_data($ty:ty) {
    impl PortData for $ty {
        unsafe fn read(port: u16) -> Self {
            Self::from_bytes(<[u8; 1]>::read(port))
        }
        unsafe fn write(port: u16, value: Self) {
            <[u8; 1]>::write(port, value.into_bytes())
        }
    }
}

#[bitfield(bits = 8)]
#[derive(Clone, Copy, Debug)]
pub struct IntrEnable {
    pub data_available: bool,
    pub tx_empty: bool,
    pub break_error: bool,
    pub modem_status: bool,
    pub sleep_mode: bool,
    pub low_power_mode: bool,
    #[skip]
    __: B2,
}
port_data!(IntrEnable);

#[derive(BitfieldSpecifier)]
#[bits = 2]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerLevel {
    Bytes1 = 0,
    Bytes4_16 = 1,
    Bytes8_32 = 2,
    Bytes14_56 = 3,
}

#[derive(BitfieldSpecifier)]
#[bits = 3]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntrType {
    ModemStatus = 0b000,
    TxEmpty = 0b001,
    RxAvail = 0b010,
    RxStatus = 0b011,
    TimeOut = 0b110,
}

#[derive(BitfieldSpecifier)]
#[bits = 2]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FifoStatus {
    NoFifo = 0,
    FifoError = 2,
    FifoOk = 3,
}

#[bitfield(bits = 8)]
#[derive(Clone, Copy, Debug)]
pub struct IntrIdent {
    pub intr_pending: bool,
    pub intr_type: IntrType,
    #[skip]
    __: B1,
    fifo_64b: bool,
    fifo_status: FifoStatus,
}
port_data!(IntrIdent);

#[bitfield(bits = 8)]
#[derive(Clone, Copy, Debug)]
pub struct FifoControl {
    enable: bool,
    clear_rx_fifo: bool,
    clear_tx_fifo: bool,
    dma_mode: bool,
    #[skip]
    __: B1,
    enable_64b_fifo: bool,
    trigger_level: TriggerLevel,
}
port_data!(FifoControl);

#[derive(BitfieldSpecifier, Clone, Copy, Debug, PartialEq, Eq)]
#[bits = 2]
pub enum WordLength {
    Bits5 = 0,
    Bits6 = 1,
    Bits7 = 2,
    Bits8 = 3,
}

#[derive(BitfieldSpecifier, Clone, Copy, Debug, PartialEq, Eq)]
#[bits = 3]
pub enum Parity {
    None = 0b000,
    Odd = 0b001,
    Even = 0b011,
    Mark = 0b101,
    Space = 0b111,
}

#[derive(BitfieldSpecifier, Clone, Copy, Debug, PartialEq, Eq)]
#[bits = 1]
enum StopBit {
    One = 0,
    Two = 1,
}

#[bitfield(bits = 8)]
#[derive(Clone, Copy, Debug)]
pub struct LineControl {
    word_length: WordLength,
    stop_bit: StopBit,
    parity: Parity,
    break_enable: bool,
    dlab: bool,
}
port_data!(LineControl);

#[bitfield(bits = 8)]
#[derive(Clone, Copy, Debug)]
struct LineStatus {
    data_ready: bool,
    overrun_error: bool,
    parity_error: bool,
    framing_error: bool,
    break_intr: bool,
    empty_tx: bool,
    empty_rx: bool,
    fifo_error: bool,
}
port_data!(LineStatus);

struct SerialPorts {
    data_div_low: Port<u8>,
    intr_enable_div_hig: Port<u8>,
    fifo_ctl_intr_ident: Port<u8>,
    line_ctl: Port<LineControl>,
    modem_ctl: Port<u8>,
    line_status: Port<LineStatus, ReadOnly>,
}

pub struct Serial<S = Init> {
    ports: SerialPorts,
    state: PhantomData<S>,
}

pub struct TxFullError;

pub struct RxEmptyError;

impl Serial<Init> {
    pub fn try_tx(&mut self, c: u8) -> Result<(), TxFullError> {
        if unsafe { !self.ports.line_status.read().empty_tx() } {
            Err(TxFullError)
        } else {
            unsafe { self.ports.data_div_low.write(c) };
            Ok(())
        }
    }

    pub fn try_rx(&mut self) -> Result<u8, RxEmptyError> {
        if unsafe { self.ports.line_status.read().empty_rx() } {
            Err(RxEmptyError)
        } else {
            let c = unsafe { self.ports.data_div_low.read() };
            Ok(c)
        }
    }

    pub fn tx_blocking(&mut self, c: u8) {
        while self.try_tx(c).is_err() {
            spin_loop();
        }
    }

    pub fn rx_blocking(&mut self) -> u8 {
        loop {
            if let Ok(c) = self.try_rx() {
                break c;
            }
            spin_loop();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rate {
    _115200,
}

impl Rate {
    fn divisor(&self) -> (u8, u8) {
        match self {
            Self::_115200 => (0x00, 0x01),
        }
    }
}

impl Serial<Uninit> {
    pub const fn new(base: u16) -> Self {
        Self {
            ports: SerialPorts {
                data_div_low: Port::new(base),
                intr_enable_div_hig: Port::new(base + 1),
                fifo_ctl_intr_ident: Port::new(base + 2),
                line_ctl: Port::new(base + 3),
                modem_ctl: Port::new(base + 4),
                line_status: Port::new(base + 5),
            },
            state: PhantomData,
        }
    }

    pub const fn com1() -> Self {
        Self::new(COM1_BASE)
    }

    pub unsafe fn update_intr_enable<R>(&mut self, f: impl FnOnce(&mut IntrEnable) -> R) -> R {
        self.intr_enable_mut().update(f)
    }

    pub unsafe fn set_intr_enable<R>(&mut self, intr_enable: IntrEnable) {
        self.intr_enable_mut().write(intr_enable)
    }

    pub unsafe fn set_rate(&mut self, rate: Rate) {
        let (h, l) = rate.divisor();
        self.ports
            .line_ctl
            .update(|line_ctl| line_ctl.set_dlab(true));
        self.ports.data_div_low.write(l);
        self.div_hig_mut().write(h);
        self.ports
            .line_ctl
            .update(|line_ctl| line_ctl.set_dlab(false));
    }

    pub unsafe fn update_line_ctl<R>(&mut self, f: impl FnOnce(&mut LineControl) -> R) -> R {
        self.ports.line_ctl.update(|line_ctl| {
            let res = f(line_ctl);
            line_ctl.set_dlab(false);
            res
        })
    }

    pub unsafe fn set_line_ctl(&mut self, line_ctl: LineControl) {
        self.ports.line_ctl.write(line_ctl)
    }

    pub unsafe fn set_fifo_ctl(&mut self, fifo_ctl: FifoControl) {
        self.fifo_ctl_mut().write(fifo_ctl)
    }

    pub unsafe fn get_intr_ident(&mut self) -> IntrIdent {
        self.intr_ident_mut().read()
    }

    pub unsafe fn defaults(mut self) -> Serial<Init> {
        self.intr_enable_mut().write(IntrEnable::new());
        self.set_rate(Rate::_115200);
        self.set_line_ctl(LineControl::new().with_word_length(WordLength::Bits8));
        self.set_fifo_ctl(
            FifoControl::new()
                .with_trigger_level(TriggerLevel::Bytes4_16)
                .with_enable_64b_fifo(true)
                .with_clear_tx_fifo(true)
                .with_clear_rx_fifo(true)
                .with_enable(true),
        );
        self.finish()
    }

    pub unsafe fn finish(mut self) -> Serial<Init> {
        self.ports.modem_ctl.write(0x0b);
        Serial {
            ports: self.ports,
            state: PhantomData,
        }
    }

    pub unsafe fn assume_init(self) -> Serial<Init> {
        Serial {
            ports: self.ports,
            state: PhantomData,
        }
    }

    fn intr_enable_mut(&mut self) -> &mut Port<IntrEnable> {
        unsafe { transmute(&mut self.ports.intr_enable_div_hig) }
    }

    fn div_hig_mut(&mut self) -> &mut Port<u8> {
        &mut self.ports.intr_enable_div_hig
    }

    fn fifo_ctl_mut(&mut self) -> &mut Port<FifoControl, WriteOnly> {
        unsafe { transmute(&mut self.ports.fifo_ctl_intr_ident) }
    }

    fn intr_ident_mut(&mut self) -> &mut Port<IntrIdent, ReadOnly> {
        unsafe { transmute(&mut self.ports.fifo_ctl_intr_ident) }
    }
}

impl fmt::Write for Serial<Init> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for &c in s.as_bytes() {
            self.tx_blocking(c)
        }
        Ok(())
    }
}

enum SerialDynState {
    Uninit(Serial<Uninit>),
    Init(Serial<Init>),
}

pub struct SerialDyn {
    state: MaybeUninit<SerialDynState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlreadyInitializedError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotInitializedError;

impl SerialDyn {
    pub const fn new(port: u16) -> Self {
        Self {
            state: MaybeUninit::new(SerialDynState::Uninit(Serial::new(port))),
        }
    }

    pub const fn com1() -> Self {
        Self::new(COM1_BASE)
    }

    pub unsafe fn defaults(&mut self) -> Result<(), AlreadyInitializedError> {
        match replace(&mut self.state, MaybeUninit::uninit()).assume_init() {
            SerialDynState::Uninit(serial) => {
                self.state = MaybeUninit::new(SerialDynState::Init(serial.defaults()));
                Ok(())
            }
            SerialDynState::Init(serial) => {
                self.state = MaybeUninit::new(SerialDynState::Init(serial));
                Err(AlreadyInitializedError)
            }
        }
    }

    pub unsafe fn finish(&mut self) -> Result<(), AlreadyInitializedError> {
        match replace(&mut self.state, MaybeUninit::uninit()).assume_init() {
            SerialDynState::Uninit(serial) => {
                self.state = MaybeUninit::new(SerialDynState::Init(serial.finish()));
                Ok(())
            }
            SerialDynState::Init(serial) => {
                self.state = MaybeUninit::new(SerialDynState::Init(serial));
                Err(AlreadyInitializedError)
            }
        }
    }

    pub unsafe fn assume_init(&mut self) -> Result<(), AlreadyInitializedError> {
        match replace(&mut self.state, MaybeUninit::uninit()).assume_init() {
            SerialDynState::Uninit(serial) => {
                self.state = MaybeUninit::new(SerialDynState::Init(serial.assume_init()));
                Ok(())
            }
            SerialDynState::Init(serial) => {
                self.state = MaybeUninit::new(SerialDynState::Init(serial));
                Err(AlreadyInitializedError)
            }
        }
    }

    pub fn as_uninit_mut(&mut self) -> Result<&mut Serial<Uninit>, AlreadyInitializedError> {
        match unsafe { self.state.assume_init_mut() } {
            SerialDynState::Uninit(serial) => Ok(serial),
            SerialDynState::Init(_) => Err(AlreadyInitializedError),
        }
    }

    pub fn as_init_mut(&mut self) -> Result<&mut Serial<Init>, NotInitializedError> {
        match unsafe { self.state.assume_init_mut() } {
            SerialDynState::Init(serial) => Ok(serial),
            SerialDynState::Uninit(_) => Err(NotInitializedError),
        }
    }
}

impl fmt::Write for SerialDyn {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.as_init_mut().map_err(|_| fmt::Error)?.write_str(s)
    }
}

pub const COM1_BASE: u16 = 0x3f8;
pub const COM2_BASE: u16 = 0x2f8;
pub const COM3_BASE: u16 = 0x3e8;
pub const COM4_BASE: u16 = 0x2e8;
