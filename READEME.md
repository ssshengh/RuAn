# 介绍
一个用于实验各种新技术以及 Rust 同 Android 协同开发的项目。
* 安卓侧主要测试技术：
1. Compose & MVI 架构
2. Kotlin 协程
3. UI 测试 & profile
* Rust 侧主要测试技术：
1. Rust 协程 & trait based 设计模式
2. FFI 自动化构建以及 profile
3. FFI 通信架构测试

# Rust FFI 编译

```shell
# release 包
just rust_sdk/build_android_all_release
```
```shell
# debug 包
just rust_sdk/build_android_all_debug
```