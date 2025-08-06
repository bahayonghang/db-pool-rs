"""
测试运行器 - 运行所有Rust和Python测试
"""

use std::process::Command;
use std::env;

fn main() {
    println!("🚀 DB-Pool-RS 测试套件");
    println!("=" * 50);
    
    // 设置测试环境
    env::set_var("RUST_LOG", "debug");
    env::set_var("DB_POOL_MODE", "simple");
    
    let mut all_passed = true;
    
    // 运行Rust单元测试
    println!("\n📦 运行Rust单元测试...");
    let rust_result = run_rust_tests();
    if !rust_result {
        all_passed = false;
    }
    
    // 运行Python测试
    println!("\n🐍 运行Python测试...");
    let python_result = run_python_tests();
    if !python_result {
        all_passed = false;
    }
    
    // 运行集成测试
    println!("\n🔗 运行集成测试...");
    let integration_result = run_integration_tests();
    if !integration_result {
        all_passed = false;
    }
    
    // 总结结果
    println!("\n" + "=" * 50);
    if all_passed {
        println!("✅ 所有测试通过！");
        std::process::exit(0);
    } else {
        println!("❌ 部分测试失败！");
        std::process::exit(1);
    }
}

fn run_rust_tests() -> bool {
    let test_files = vec![
        "test_config",
        "test_pool_manager", 
        "test_types",
        "test_monitoring",
    ];
    
    let mut all_passed = true;
    
    for test_file in test_files {
        println!("  🧪 运行 {}...", test_file);
        
        let output = Command::new("cargo")
            .args(&["test", "--test", test_file])
            .output();
            
        match output {
            Ok(output) => {
                if output.status.success() {
                    println!("    ✅ {} 通过", test_file);
                } else {
                    println!("    ❌ {} 失败", test_file);
                    println!("    错误输出: {}", String::from_utf8_lossy(&output.stderr));
                    all_passed = false;
                }
            }
            Err(e) => {
                println!("    ❌ {} 执行失败: {}", test_file, e);
                all_passed = false;
            }
        }
    }
    
    all_passed
}

fn run_python_tests() -> bool {
    println!("  🧪 运行Python测试套件...");
    
    let output = Command::new("python")
        .args(&["-m", "pytest", "tests/python/", "-v"])
        .output();
        
    match output {
        Ok(output) => {
            if output.status.success() {
                println!("    ✅ Python测试通过");
                true
            } else {
                println!("    ❌ Python测试失败");
                println!("    错误输出: {}", String::from_utf8_lossy(&output.stderr));
                false
            }
        }
        Err(e) => {
            println!("    ❌ Python测试执行失败: {}", e);
            false
        }
    }
}

fn run_integration_tests() -> bool {
    println!("  🧪 运行集成测试...");
    
    // 运行基础示例作为集成测试
    let output = Command::new("python")
        .args(&["examples/python/basic_usage.py"])
        .output();
        
    match output {
        Ok(output) => {
            if output.status.success() {
                println!("    ✅ 集成测试通过");
                true
            } else {
                println!("    ❌ 集成测试失败");
                println!("    错误输出: {}", String::from_utf8_lossy(&output.stderr));
                false
            }
        }
        Err(e) => {
            println!("    ❌ 集成测试执行失败: {}", e);
            false
        }
    }
}