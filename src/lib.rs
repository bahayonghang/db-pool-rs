use pyo3::prelude::*;

pub mod core;
pub mod databases;
pub mod python;
pub mod utils;

use python::pool::PyDatabasePool;

/// db-pool-rs Python模块
#[pymodule]
fn _db_pool_rs(_py: Python, m: &PyModule) -> PyResult<()> {
    // 添加版本信息
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    
    // 添加核心类 (导出为 DatabasePool)
    m.add_class::<PyDatabasePool>()?;
    
    // 为Python兼容性添加别名
    m.add("DatabasePool", _py.get_type::<PyDatabasePool>())?;
    
    // 添加异常类型
    m.add("DbPoolError", _py.get_type::<pyo3::exceptions::PyRuntimeError>())?;
    
    Ok(())
}