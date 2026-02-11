package com.colin.tauri_airdrop

import android.content.Context
import android.net.wifi.WifiManager
import android.util.Log

class MulticastManager(private val context: Context) {
    private var multicastLock: WifiManager.MulticastLock? = null

    fun acquire() {
        try {
            val wifiManager = context.applicationContext
                .getSystemService(Context.WIFI_SERVICE) as WifiManager

            multicastLock = wifiManager.createMulticastLock("airdrop_multicast")
            multicastLock?.acquire()

            Log.d(TAG, "Multicast lock acquired successfully")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to acquire multicast lock", e)
        }
    }

    fun release() {
        try {
            multicastLock?.release()
            multicastLock = null
            Log.d(TAG, "Multicast lock released")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to release multicast lock", e)
        }
    }

    companion object {
        private const val TAG = "MulticastManager"
    }
}
