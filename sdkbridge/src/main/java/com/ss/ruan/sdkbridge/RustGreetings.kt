package com.ss.ruan.sdkbridge

class RustGreetings {
    init {
        System.loadLibrary("jni_sdk")
    }
    
    private external fun greeting(pattern: String): String

    fun sayHello(to: String): String {
        return greeting(to)
    }
}