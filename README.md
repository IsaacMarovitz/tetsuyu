# tetsuyu

A GameBoy emulator, written in Rust.

### Features
- Supports MBC1/MBC2/MBC3/MBC5 DMG Titles
- Sprite & BG Rendering
- Input
- Cycle-Accurate CPU
- Cross-Platform
- Cross-Graphics API (Metal, Vulkan, OpenGL, D3D12, WebGPU)

### Partially Complete
- [ ] Channel 1
  - [x] Period
  - [x] Duty Cycle
  - [ ] Sweep
  - [ ] Envelope
- [ ] Channel 2
  - [x] Period
  - [x] Duty Cycle
  - [ ] Envelope
- [ ] Channel 3
  - [ ] Wave
  - [ ] Period
- [ ] Channel 4
  - [ ] LFSR
        
### Not Complete
- CGB Support
- UI/Config Settings
- Custom Palettes

<img width="400" alt="cpu_instrs Test" src="https://github.com/IsaacMarovitz/tetsuyu/assets/42140194/a1b62888-0efa-4132-93fe-7ee812f7c73e">
<img width="400" alt="instr_timing Test" src="https://github.com/IsaacMarovitz/tetsuyu/assets/42140194/56fe26c1-cc4b-498e-9fd0-26a3d109c0ba">


### Referenced Documentation
- https://gbdev.io/pandocs/CPU_Instruction_Set.html

### Referenced Implementations
- https://github.com/mvdnes/rboy
- https://github.com/mohanson/gameboy
