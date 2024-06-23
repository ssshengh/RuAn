package com.ss.ruan.sdkbridge

import android.util.Log
import java.util.logging.Logger

class SdkLogger {
    companion object {
        private val STATIC_LOGGER = object : ILogger {
            override fun onLog(category: String, level: Byte, msg: String) {
                when (level.toInt()) {
                    1 -> Log.e(category, msg)
                    2 -> Log.w(category, msg)
                    3 -> Log.i(category, msg)
                    4 -> Log.d(category, msg)
                    5 -> Log.v(category, msg)
                    else -> Log.i(category, msg)
                }
            }
        }
    }
    
    init {
        System.loadLibrary("jni_sdk")
    }
    
    private external fun initLog(logCallback: ILogger)
    
    fun initLogger() {
        initLog(STATIC_LOGGER)
    }
    
    interface ILogger {
        fun onLog(category: String, level: Byte, msg: String)
    }
}