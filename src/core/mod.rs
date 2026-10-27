//! 核心装配流水线：哈希 → 部件编号 → 角色/主题 → 着色 → 拼装 → 优化。
//!
//! 设计模式对应：策略（哈希/渲染）、模板方法（拼装顺序、形状绘制）、
//! 不可变对象（AvatarSpec/Avatar）、简单工厂（渲染器）、单例（数据表）。

pub mod avatar_id_hasher;
pub mod part_number_mapper;
pub mod part_key_resolver;
pub mod avatar_spec;
pub mod svg_composer;
pub mod svg_fragment_painter;
pub mod svg_optimizer;

pub use avatar_id_hasher::{AvatarIdHasher, Sha256AvatarIdHasher};
pub use avatar_spec::AvatarSpec;
pub use svg_composer::SvgComposer;
pub use svg_fragment_painter::paint as paint_fragment;
pub use svg_optimizer::{optimize as optimize_svg, round_token};
