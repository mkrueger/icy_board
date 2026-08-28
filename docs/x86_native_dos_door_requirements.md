# x86-native requirements for FreeDOS and BBS doors

## Purpose

This document lists what [`x86-native`](https://github.com/illussioon/x86) needs
to support before it can serve as a reusable native runtime for classic DOS BBS
doors.

The target is a native Rust library. It must not require a browser, JavaScript,
Node.js, DOSBox, QEMU or DOSEMU at runtime.

The primary acceptance path is:

```text
SeaBIOS
  -> boot writable FreeDOS HDD
  -> load X00-compatible FOSSIL driver
  -> launch DOS door with drop file
  -> exchange bytes through COM1
  -> door exits
  -> FDAPM powers machine off
  -> disk changes persist
```

Icy Board is an intended consumer, but the APIs described here should remain BBS
and application independent.

## Current useful foundation

As of `x86-native` 0.1.6, the project already provides or has begun providing:

- a native Rust port of the v86 interpreter
- x86, FPU, SSE and paging implementation
- v86 saved-state parsing and partial restoration
- a public `Machine` and `ExecutionBackend` abstraction
- BIOS, VGA BIOS, raw disk and saved-state image types
- PIC/APIC and IOAPIC state support
- partial PIT, RTC, PS/2 and UART models
- VGA framebuffer and text-plane extraction
- native VirtIO/9P work
- Linux, Windows and macOS CI/release infrastructure
- BSD-2-Clause or MIT licensing

The missing work is mainly cold boot, complete DOS-relevant devices, reusable
serial and disk interfaces, lifecycle reporting and machine isolation.

## Priority summary

| Priority | Capability | Why it blocks DOS doors |
| :--- | :--- | :--- |
| P0 | Machine resources passed into backend | BIOS and disks currently cannot drive native execution cleanly |
| P0 | Per-machine state or enforced isolation | Current globals prevent safe concurrent sessions |
| P0 | Bidirectional COM1 host API | BBS connections and FOSSIL doors communicate through serial bytes |
| P0 | BIOS ROM mapping and reset boot | FreeDOS must cold boot rather than only resume a saved state |
| P0 | Writable IDE raw disk | Door installation and game state live on the DOS hard disk |
| P0 | PIC, PIT, RTC and IRQ correctness | DOS, BIOS and FOSSIL drivers depend on timers and interrupts |
| P1 | DMA and floppy controller | Session drop files and `RUN.BAT` are most easily injected on `A:` |
| P1 | APM/ACPI power-off reporting | The host needs a reliable clean door-session exit |
| P1 | Cancellation and resource limits | A hung or hostile guest must not block the BBS |
| P1 | Disk flush/overlay policy | Door state must persist without corrupting the base image |
| P2 | Saved-state startup acceleration | Cold boot latency should not delay every caller |
| P2 | VGA setup support | Useful for DOS setup programs, not required for serial doors |

## 1. Pass machine resources to the execution backend

### Problem

`Machine` owns BIOS and disk `Image` values, but `ExecutionBackend::reset`
receives only `MachineConfig`. A native backend cannot reliably cold boot the
resources attached through `Machine::set_bios`, `set_vga_bios` and `set_disk`.

### Required API

Introduce a resource view passed during backend preparation:

```rust
pub struct MachineResources<'a> {
    pub bios: Option<&'a Image>,
    pub vga_bios: Option<&'a Image>,
    pub hard_disks: &'a [DiskAttachment],
    pub floppy_disks: &'a [DiskAttachment],
    pub cdroms: &'a [Image],
}

pub trait ExecutionBackend: Send {
    fn prepare(
        &mut self,
        config: &MachineConfig,
        resources: &MachineResources<'_>,
    ) -> Result<()>;
}
```

An owned resource configuration is also acceptable if the backend must retain
mutable disks. The important contract is that resources configured on `Machine`
reach the backend without downcasting or process-global registries.

### Acceptance tests

- A fake backend receives the exact BIOS and disks attached to `Machine`.
- Missing required resources return typed errors.
- A backend can retain writable disk handles without unsafe lifetime tricks.
- Existing saved-state users remain supported.

## 2. Replace process-global machine state

### Problem

The native runtime currently uses global pointers and `OnceLock` device state for
UART, PIT, RTC, PS/2 and VGA. `NativeBackend` therefore documents itself as
single-machine.

A BBS may run many independent doors concurrently. Global state risks memory,
interrupt, serial and timer leakage between callers.

### Required architecture

Each backend owns all mutable state:

```rust
pub struct NativeRuntime {
    cpu: Cpu,
    memory: Memory,
    io: IoBus,
    mmio: MmioBus,
    devices: DeviceSet,
    clock: Box<dyn Clock>,
    entropy: Box<dyn Entropy>,
}
```

No machine-specific state may remain in process-global mutable statics.
Read-only generated instruction tables may remain static.

If immediate refactoring is too large, explicitly support one machine per
process as a transitional mode. Do not expose multiple `NativeBackend` instances
as safe until isolation tests pass.

### Acceptance tests

- Two machines execute different real-mode programs on separate threads.
- Each writes a different byte stream to COM1.
- Memory, registers, IRQs, timers and serial queues do not cross machines.
- Dropping one machine does not invalidate another.
- ThreadSanitizer or an equivalent concurrency test reports no shared-state race.

## 3. Stable byte-oriented serial host API

### Problem

The current UART writes output directly to process stdout and keeps input in an
internal global queue. BBS software needs raw byte I/O, modem status and
backpressure without terminal encoding inside the emulator.

### Required API

Expose serial ports through either a backend trait or explicit machine methods:

```rust
pub trait SerialBackend: Send {
    fn receive_byte(&mut self) -> Option<u8>;
    fn transmit_byte(&mut self, value: u8);
    fn modem_status(&self) -> ModemStatus;
}

#[derive(Clone, Copy)]
pub struct ModemStatus {
    pub carrier_detect: bool,
    pub data_set_ready: bool,
    pub clear_to_send: bool,
    pub ring_indicator: bool,
}
```

Alternative pull API:

```rust
machine.serial_input(0, input)?;
let count = machine.serial_output(0, output)?;
machine.set_modem_status(0, status)?;
```

Required behavior:

- COM1 base port `0x3F8`, IRQ 4
- raw 8-bit input and output
- divisor latch
- line control
- modem control and status
- interrupt enable and identification
- receive-data-ready interrupt
- transmit-holding-register-empty interrupt
- line-status register
- scratch register
- FIFO behavior sufficient for 16550-aware software
- configurable DCD, DSR and CTS
- bounded input/output queues or host backpressure

Do not print inside the UART implementation.

### Acceptance tests

- Guest `OUT` to COM1 appears in host output.
- Host byte appears in the guest receive register.
- RX interrupt fires only when enabled.
- TX-empty interrupt behavior matches a 16550.
- DCD, DSR and CTS bits are visible to the guest.
- Divisor-latch writes do not emit serial bytes.
- A sustained bidirectional transfer loses no bytes.

## 4. Cold boot from BIOS

### Required behavior

- Start at the x86 reset vector.
- Map system BIOS at the expected high-memory aliases.
- Map VGA BIOS at `0xC0000` and any required PCI ROM address.
- Supply CMOS memory and disk geometry expected by SeaBIOS.
- Implement A20 behavior used during boot.
- Route BIOS port and MMIO access through native devices.
- Support boot order selection between HDD and floppy.

### Acceptance tests

- SeaBIOS executes from reset without a saved state.
- SeaBIOS detects configured RAM.
- SeaBIOS detects one IDE disk and one floppy.
- SeaBIOS selects HDD-first when requested.
- Unknown I/O accesses are traceable and fail predictably.

## 5. DOS-relevant CPU correctness

The interpreter should retain upstream v86 behavior for:

- 16-bit real mode
- 16-bit and 32-bit protected mode used by extenders
- paging
- x87 FPU
- common 386/486/Pentium instructions
- string instructions and REP prefixes
- interrupt and exception delivery
- segment limits and privilege checks
- virtual 8086 mode where required
- RDTSC behavior used by timing code

A JIT is not required initially. Interpreter performance only needs to support
interactive text doors acceptably.

### Acceptance tests

- Reuse upstream v86 CPU fixtures.
- Differentially compare registers and memory against upstream v86.
- Run representative DOS extender and FPU diagnostics.
- Run two CPU instances concurrently.

## 6. PIC, PIT, RTC and timers

### PIC

- master and slave 8259
- masking and acknowledgement
- edge-triggered IRQ behavior
- cascade handling
- IRQ 0, 1, 4, 6 and 14 at minimum

### PIT

- channels 0-2
- modes required by BIOS and DOS
- latch/read-back behavior
- IRQ0 generation
- deterministic clock injection for tests

### RTC/CMOS

- ports `0x70`/`0x71`
- current date and time
- status registers
- periodic/update interrupts where used
- memory size
- floppy type
- disk geometry and boot flags

### Required host abstraction

```rust
pub trait Clock: Send {
    fn monotonic_micros(&self) -> u64;
    fn wall_clock(&self) -> SystemTime;
}
```

Avoid `Instant::now()` inside global device state. A fake clock must be usable in
unit tests.

### Acceptance tests

- BIOS timer ticks advance.
- DOS `TIME` and `DATE` are valid.
- UART IRQ4 is delivered while timer IRQ0 remains active.
- Fixed-clock tests produce repeatable interrupt order.

## 7. Writable raw IDE disks

### Public disk abstraction

```rust
pub trait Disk: Send {
    fn sector_size(&self) -> u32;
    fn sector_count(&self) -> u64;
    fn read_sector(&mut self, sector: u64, output: &mut [u8]) -> Result<()>;
    fn write_sector(&mut self, sector: u64, input: &[u8]) -> Result<()>;
    fn flush(&mut self) -> Result<()>;
}
```

Initial backends:

- memory-backed disk
- file-backed raw disk
- read-only wrapper
- copy-on-write overlay

### IDE behavior

- primary channel at standard ports
- one master ATA disk initially
- IDENTIFY
- CHS and LBA28 reads/writes
- status/error registers
- IRQ14
- PIO transfers
- flush command
- reset behavior

DMA IDE is not needed for the first DOS profile unless FreeDOS/SeaBIOS requires
it for the selected setup.

### Acceptance tests

- SeaBIOS identifies the disk.
- FreeDOS boots from a raw image.
- Guest reads and writes sectors.
- A file created in DOS survives poweroff and reboot.
- Read-only and overlay disks reject or isolate writes correctly.
- Out-of-range requests return ATA errors without host panics.

## 8. DMA and floppy controller

A per-session floppy is the simplest way to inject drop files and `RUN.BAT`
without modifying the shared DOS image.

Required support:

- Intel 8237-style DMA behavior needed by FDC
- floppy controller at `0x3F0`
- IRQ6 and DMA channel 2
- 1.44 MiB FAT12 images
- media detection through CMOS
- sector read support
- sector write support if setup tools need it
- HDD-first boot while floppy remains mounted as `A:`

### Acceptance tests

- SeaBIOS sees a 1.44 MiB floppy.
- FreeDOS can `DIR A:`.
- FreeDOS copies `A:\DOOR.SYS` and `A:\RUN.BAT` to the HDD.
- A nonbootable session floppy does not override HDD-first boot.

## 9. Minimal VGA required by SeaBIOS

Headless serial doors do not require a graphical UI, but SeaBIOS may require
basic VGA behavior.

Required minimum:

- VGA BIOS mapping
- legacy text memory
- required VGA I/O registers
- enough Bochs VBE/PCI behavior for SeaBIOS initialization
- optional text snapshot for diagnostics

Framebuffer rendering is useful but not a blocker for COM1 doors.

### Acceptance tests

- SeaBIOS initializes without hanging on VGA access.
- VGA text snapshot shows BIOS or DOS diagnostics.
- Headless mode does not allocate unnecessary large graphical buffers.

## 10. APM/ACPI poweroff and machine exits

### Required API

```rust
pub enum MachineExit {
    PowerOff,
    Reset,
    Cancelled,
    InstructionLimit,
    WallClockLimit,
    TripleFault,
    Fatal(MachineError),
}
```

Support the poweroff mechanism used by FreeDOS `FDAPM POWEROFF`. Distinguish
poweroff from reset, HLT and temporary guest idle.

`ExecutionBackend::step()` should return more than a boolean:

```rust
pub struct StepReport {
    pub instructions: u64,
    pub state: ExecutionState,
}

pub enum ExecutionState {
    Running,
    HaltedWaitingForInterrupt,
    ResetRequested,
    PowerOffRequested,
    Fatal,
}
```

### Acceptance tests

- `HLT` waits for interrupts and does not end the machine.
- `FDAPM POWEROFF` returns `MachineExit::PowerOff`.
- Guest reset is distinguishable from poweroff.
- Triple fault is reported.

## 11. Cancellation and limits

Required controls:

- atomic cancellation token
- instruction budget
- wall-clock timeout
- guest RAM limit
- VGA RAM limit
- disk size limit
- serial queue limit
- maximum instructions per host quantum

Cancellation must work when the guest:

- is executing continuously
- is halted waiting for an interrupt
- loops in BIOS
- floods COM1 output

### Acceptance tests

- Cancellation stops each state within a bounded time.
- Queue limits apply backpressure or return a typed error.
- A malformed guest cannot allocate unbounded host memory.

## 12. Disk persistence and crash policy

The library should expose mechanisms, not impose a BBS policy.

Required operations:

- explicit disk flush
- atomic image replacement helper
- copy-on-write overlay export
- dirty-state query
- optional sector-change journal

Do not advertise one writable FAT image shared by independent virtual machines
as safe. Separate DOS kernels do not share file-lock state or filesystem caches.

### Acceptance tests

- Clean poweroff flushes all modified sectors.
- Overlay mode leaves the base image unchanged.
- Exported overlay reproduces guest changes.
- Interrupted atomic replacement retains either old or new complete image.

## 13. Saved-state restore and creation

Saved states are valuable for reducing door startup time, but they come after a
working cold boot.

Required eventually:

- restore complete CPU and DOS-profile device state
- save complete CPU, memory and device state
- versioned state format
- validation against machine resources
- replaceable session floppy after restore
- reset serial queues and modem status per session

Recommended snapshot point:

```text
FreeDOS booted -> FOSSIL loaded -> immediately before CALL A:\RUN.BAT
```

### Acceptance tests

- Cold boot and restored boot produce equivalent door behavior.
- Restored IDE/FDC state matches attached images.
- No prior caller's serial input/output survives restore.
- Snapshot incompatibility returns a typed error.

## 14. Error and tracing API

Use typed errors instead of log strings:

```rust
pub enum MachineError {
    InvalidImage { device: DeviceKind, reason: String },
    UnsupportedPort { port: u16, width: AccessWidth },
    UnsupportedMmio { address: u64, width: AccessWidth },
    DeviceFault { device: DeviceKind, reason: String },
    DiskIo { device: usize, source: std::io::Error },
    GuestFault(GuestFault),
}
```

Tracing should be optional and consumer-neutral:

- I/O port access
- MMIO access
- IRQ raise/lower/acknowledge
- disk command and sector
- UART register and bytes
- reset/poweroff
- unsupported-device access

Use the `log` or `tracing` facade behind features; do not initialize a subscriber
inside the library.

## 15. Threading model

The core remains synchronous. A consumer may run one `Machine` on one blocking
thread and communicate through bounded channels.

Requirements:

- `Machine: Send`
- no requirement for `Machine: Sync`
- no Tokio dependency in core crates
- clear statement that one thread drives one machine at a time
- cancellation usable from another thread
- serial adapters may bridge to async runtimes externally

## 16. FAT tooling

FAT manipulation belongs in an optional utility crate, not the CPU/device core.

Suggested commands/API:

```text
v86-fat ls IMAGE [PATH]
v86-fat read IMAGE DOS_PATH
v86-fat copy IMAGE HOST_PATH DOS_PATH
v86-fat mkdir IMAGE DOS_PATH
v86-fat remove IMAGE DOS_PATH
v86-fat create-floppy OUTPUT FILE...
```

Using the Rust `fatfs` crate is sufficient initially.

Required in-memory helper:

```rust
pub fn create_fat12_floppy(
    files: impl IntoIterator<Item = DosFile>,
) -> Result<MemoryDisk>;
```

Validate DOS 8.3 file names and normalize line endings for generated batch files
at the caller layer.

## 17. FreeDOS/FOSSIL compatibility profile

Expose a named configuration profile:

```rust
let config = MachineConfig::freedos_door()
    .with_memory_mb(32)
    .with_boot_order(BootOrder::HardDiskFirst);
```

Recommended defaults:

- one CPU
- 32 MiB RAM
- COM1 enabled
- DCD, DSR and CTS asserted
- primary IDE master enabled
- one 1.44 MiB floppy
- networking disabled
- audio disabled
- HDD-first boot
- APM/ACPI poweroff enabled
- deterministic unsupported-device errors

## 18. Differential and integration tests

Use upstream v86 as an oracle where practical.

Compare:

- CPU registers
- selected memory ranges
- COM1 byte stream
- interrupt order
- I/O port trace
- disk sector changes
- reset/poweroff result

Test artifacts:

- handcrafted instruction binaries
- serial boot sector
- PIT/IRQ boot sector
- ATA read/write `.COM`
- UART interrupt `.COM`
- FOSSIL diagnostic
- redistributable FreeDOS image or CI-downloaded fixture
- small open-source test door

Do not commit proprietary LORD, TradeWars or PimpWars binaries. Permit ignored
local compatibility fixtures.

## 19. CI requirements

Required CI targets:

- Linux x86_64
- Windows x86_64
- macOS x86_64
- macOS ARM64

Required checks:

- `cargo fmt --check`
- clippy
- all unit tests
- two-machine isolation test
- boot-sector COM1 test
- raw IDE persistence test
- FreeDOS smoke test where licensing permits
- FOSSIL serial test
- fuzz targets for image parsing and device access

Keep long boot tests in a separate job if needed.

## 20. Suggested contribution sequence

Each item should be a reviewable upstream PR.

### PR 1: machine resource handoff

- Add `MachineResources` or equivalent.
- Pass attached BIOS/disks to `ExecutionBackend`.
- Add fake-backend tests.

This unblocks all cold-boot device work.

### PR 2: reusable serial boundary

- Remove direct UART stdout writes.
- Add host input/output queues or callback interface.
- Add modem status API.
- Test raw bidirectional bytes.

This gives BBS consumers an early usable integration boundary.

### PR 3: explicit execution results and cancellation

- Replace `step() -> bool` with a structured result.
- Add cancellation and instruction limits.
- Distinguish HLT, reset, poweroff and fatal errors.

### PR 4: instance-owned host devices

- Move UART, PIT, RTC, PS/2 and VGA state out of `OnceLock` globals.
- Establish `NativeRuntime`/`DeviceSet` ownership.
- Add two-machine concurrency tests.

Coordinate this design with the maintainer before implementation because it is a
large internal API change.

### PR 5: BIOS cold boot

- Map BIOS and VGA BIOS.
- Add reset-vector and SeaBIOS smoke tests.
- Implement required CMOS/chipset behavior.

### PR 6: disk abstraction and primary IDE

- Add `Disk` trait and memory/file/overlay disks.
- Implement ATA PIO read/write and IDENTIFY.
- Boot FreeDOS from raw HDD.

### PR 7: DMA and floppy

- Implement DMA and FDC support.
- Mount an in-memory FAT12 session floppy as `A:`.

### PR 8: DOS poweroff

- Implement the shutdown path used by `FDAPM POWEROFF`.
- Return `MachineExit::PowerOff`.

### PR 9: FOSSIL acceptance

- Complete 16550 interrupt/FIFO behavior.
- Run X00 and a FOSSIL diagnostic.

### PR 10: complete DOS-door demonstration

- Build a session floppy with `DOOR.SYS` and `RUN.BAT`.
- Launch a redistributable test door.
- Bridge COM1.
- Persist HDD state.
- Power off cleanly.

### PR 11: snapshots and startup performance

- Save complete DOS-profile state.
- Restore before `RUN.BAT`.
- Benchmark cold versus restored startup.

## 21. Definition of done for x86-native DOS-door support

The upstream library is ready for BBS integration when all statements are true:

- [ ] A machine can cold boot SeaBIOS.
- [ ] FreeDOS boots from a writable raw IDE image.
- [ ] A FAT12 session floppy is mounted as `A:`.
- [ ] COM1 supports raw bidirectional bytes.
- [ ] DCD, DSR and CTS are visible to the guest.
- [ ] X00 or another FOSSIL driver passes a serial diagnostic.
- [ ] A DOS door reads a generated drop file.
- [ ] Door output reaches the host through COM1.
- [ ] Host input reaches the door through COM1.
- [ ] `FDAPM POWEROFF` returns `MachineExit::PowerOff`.
- [ ] Clean disk changes survive reboot.
- [ ] Cancellation stops a hung guest promptly.
- [ ] Malformed images and guest operations do not panic the host.
- [ ] Two machine instances run without shared CPU/device state.
- [ ] The public API contains no Icy Board-specific types.
- [ ] Linux, Windows and macOS CI pass.

## 22. Icy Board work that should remain downstream

These features belong in Icy Board, not `x86-native`:

- generating PCBoard/BBS drop-file formats
- deciding which user and conference fields enter a drop file
- rendering `RUN.BAT` substitutions such as `{node}` and `{dropFile}`
- mapping an Icy Board connection to a serial adapter
- sysop monitoring and door output tracking
- door security/password checks
- per-door concurrency policy
- image path resolution under the board root
- board activity and accounting

`x86-native` should provide the machine, disks, serial ports, lifecycle and exit
status. The BBS should provide policy and session data.

## First concrete target

The best first target is smaller than FreeDOS:

```text
Two NativeBackend instances
  -> each executes a distinct 16-bit boot-sector program
  -> each writes a distinct message to COM1
  -> each accepts and echoes one host byte
  -> both run concurrently
  -> neither leaks state into the other
```

That test forces resource handoff, serial APIs, interrupt routing and per-machine
ownership into the design before IDE, floppy and SeaBIOS make refactoring more
expensive.
