package com.example.hybrid_2d_3d // Update with your actual package name

import android.os.Bundle
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

class MainActivity: FlutterActivity() {
    private val CHANNEL = "fluorescent_xr_channel"

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, CHANNEL).setMethodCallHandler { call, result ->
            if (call.method == "initOpenXR") {
                // Pass down the current Activity to the Rust layer
                val success = initNativeOpenXR(this)
                result.success(success)
            } else {
                result.notImplemented()
            }
        }
    }

    // JNI binding to Rust. 
    // We explicitly pass 'this' (the Activity) as a parameter.
    private external fun initNativeOpenXR(activity: Any): Boolean

    companion object {
        init {
            // Load the rust compiled shared library
            System.loadLibrary("fluoderpod_render")
        }
    }
}
