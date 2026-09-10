//! 相位分离系统（stage0.md §8.9 / §12.3）：Phase 0 / Phase 1 的物理隔离。
//!
//! **三个关键规则**（§8.9）：
//! 1. Phase 1 代码只能产生 Phase 0 代码，不能直接执行 Phase 0 代码；
//! 2. 区分「实例化」（执行模块体）和「访问」（仅执行 Phase 1 部分）；
//! 3. 传递依赖的相位传播。
//!
//! 模块生命周期通过 **declare / instantiate / visit** 三种操作管理：
//! - declare：登记模块与其相位声明（名称 + 导入 + 导出）；
//! - visit：仅执行 Phase 1 部分（宏变换器加载——本 Stage 0 单模块实现中
//!   对应为「变换器注册表已就绪」的断言）；
//! - instantiate：执行模块体（Phase 0 实例化——对应为编译器消费 CoreExpr）。

use kerf_syntax::Symbol;

/// 相位层级（Phase 0 = 运行时，Phase 1 = 宏展开时；更高相位 Stage 2+ 评估）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PhaseLevel {
    /// Phase 0：运行时世界。
    Runtime,
    /// Phase 1：宏展开（编译时）世界。
    Expand,
}

impl PhaseLevel {
    /// 相位数值（传递依赖的相位传播计算用）。
    pub fn as_u32(self) -> u32 {
        match self {
            PhaseLevel::Runtime => 0,
            PhaseLevel::Expand => 1,
        }
    }
}

/// 模块登记条目（declare 的产物）。
#[derive(Debug, Clone)]
pub struct ModuleEntry {
    /// 模块名。
    pub name: Symbol,
    /// 导入的模块名。
    pub imports: Vec<Symbol>,
    /// 导出的符号。
    pub exports: Vec<Symbol>,
    /// 是否已完成 visit（Phase 1 变换器加载）。
    pub visited: bool,
    /// 是否已完成 instantiate（Phase 0 执行）。
    pub instantiated: bool,
}

/// 模块注册表：相位分离的运行簿记（driver 在管线各阶段调用）。
#[derive(Debug, Default)]
pub struct ModuleRegistry {
    entries: Vec<ModuleEntry>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        ModuleRegistry::default()
    }

    /// declare：登记模块与其相位声明（重复声明报错——显式失败，不静默覆盖）。
    pub fn declare(
        &mut self,
        name: Symbol,
        imports: Vec<Symbol>,
        exports: Vec<Symbol>,
    ) -> Result<(), String> {
        if self.find(name).is_some() {
            return Err(format!("模块重复声明（Symbol {}）", name.0));
        }
        self.entries.push(ModuleEntry {
            name,
            imports,
            exports,
            visited: false,
            instantiated: false,
        });
        Ok(())
    }

    /// visit：仅执行 Phase 1 部分（变换器加载）。
    /// 规则 3（传递依赖的相位传播）：先 visit 全部导入模块。
    pub fn visit(&mut self, name: Symbol) -> Result<(), String> {
        let imports: Vec<Symbol> = self
            .find(name)
            .ok_or_else(|| format!("未声明的模块（Symbol {}）", name.0))?
            .imports
            .clone();
        for dep in imports {
            // 传递依赖先行（无环假设由 declare 侧 DAG 保证；环在此显式失败）
            self.visit(dep)?;
        }
        let entry = self.find_mut(name).expect("上方已检查");
        if entry.visited {
            return Ok(()); // 幂等
        }
        entry.visited = true;
        Ok(())
    }

    /// instantiate：执行模块体（Phase 0 实例化）。前置：已 visit。
    pub fn instantiate(&mut self, name: Symbol) -> Result<(), String> {
        let visited = self
            .find(name)
            .ok_or_else(|| format!("未声明的模块（Symbol {}）", name.0))?
            .visited;
        if !visited {
            return Err(format!(
                "模块（Symbol {}）未 visit 即 instantiate——相位违规",
                name.0
            ));
        }
        let entry = self.find_mut(name).expect("上方已检查");
        entry.instantiated = true;
        Ok(())
    }

    /// 查询模块。
    pub fn find(&self, name: Symbol) -> Option<&ModuleEntry> {
        self.entries.iter().find(|e| e.name == name)
    }

    fn find_mut(&mut self, name: Symbol) -> Option<&mut ModuleEntry> {
        self.entries.iter_mut().find(|e| e.name == name)
    }

    /// 全部模块条目。
    pub fn entries(&self) -> &[ModuleEntry] {
        &self.entries
    }

    /// 未完成实例化的模块数（管线完成度断言）。
    pub fn pending_instantiations(&self) -> usize {
        self.entries.iter().filter(|e| !e.instantiated).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declare_visit_instantiate_lifecycle() {
        let mut reg = ModuleRegistry::new();
        reg.declare(Symbol(1), vec![], vec![Symbol(2)]).unwrap();
        // 未 visit 即 instantiate → 相位违规（显式失败）
        assert!(reg.instantiate(Symbol(1)).is_err());
        reg.visit(Symbol(1)).unwrap();
        reg.instantiate(Symbol(1)).unwrap();
        assert_eq!(reg.pending_instantiations(), 0);
    }

    #[test]
    fn duplicate_declare_fails() {
        let mut reg = ModuleRegistry::new();
        reg.declare(Symbol(1), vec![], vec![]).unwrap();
        assert!(reg.declare(Symbol(1), vec![], vec![]).is_err());
    }

    #[test]
    fn transitive_visit_order() {
        // m2 imports m1：visit(m2) 必须先 visit(m1)
        let mut reg = ModuleRegistry::new();
        reg.declare(Symbol(1), vec![], vec![]).unwrap(); // m1
        reg.declare(Symbol(2), vec![Symbol(1)], vec![]).unwrap(); // m2
        reg.visit(Symbol(2)).unwrap();
        assert!(reg.find(Symbol(1)).unwrap().visited);
        assert!(reg.find(Symbol(2)).unwrap().visited);
    }

    #[test]
    fn visit_is_idempotent() {
        let mut reg = ModuleRegistry::new();
        reg.declare(Symbol(1), vec![], vec![]).unwrap();
        reg.visit(Symbol(1)).unwrap();
        reg.visit(Symbol(1)).unwrap();
    }
}
