import org.gradle.kotlin.dsl.support.kotlinCompilerOptions

plugins {
    id("java-library")
    alias(libs.plugins.jetbrains.kotlin.jvm)
}

repositories {
    google()
    mavenCentral()
}

java {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
}

// 需要定义 kotlin 的编译版本
kotlin {
    jvmToolchain(17)
}