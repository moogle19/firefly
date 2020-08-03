use crate::spec::{Endianness, LinkerFlavor, Target, TargetOptions, TargetResult};

pub fn target() -> TargetResult {
    let mut base = super::linux_base::opts();
    base.max_atomic_width = Some(64);
    Ok(Target {
        llvm_target: "arm-unknown-linux-gnueabi".to_string(),
        target_endian: Endianness::Little,
        target_pointer_width: 32,
        target_c_int_width: "32".to_string(),
        data_layout: "e-m:e-p:32:32-Fi8-i64:64-v128:64:128-a:0:32-n32-S64".to_string(),
        arch: "arm".to_string(),
        target_os: "linux".to_string(),
        target_env: "gnu".to_string(),
        target_vendor: "unknown".to_string(),
        linker_flavor: LinkerFlavor::Gcc,

        options: TargetOptions {
            features: "+strict-align,+v6".to_string(),
            unsupported_abis: super::arm_base::unsupported_abis(),
            target_mcount: "\u{1}__gnu_mcount_nc".to_string(),
            ..base
        },
    })
}
