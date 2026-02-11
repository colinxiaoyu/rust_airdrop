package com.colin.tauri_airdrop

import android.Manifest
import android.os.Build
import android.os.Bundle
import androidx.activity.enableEdgeToEdge
import androidx.core.app.ActivityCompat

class MainActivity : TauriActivity() {
  private lateinit var multicastManager: MulticastManager

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    // Initialize and acquire multicast lock (required for mDNS discovery)
    multicastManager = MulticastManager(this)
    multicastManager.acquire()

    // Request runtime permissions for Android 13+
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
      ActivityCompat.requestPermissions(
        this,
        arrayOf(
          Manifest.permission.POST_NOTIFICATIONS,
          Manifest.permission.READ_MEDIA_IMAGES,
          Manifest.permission.READ_MEDIA_VIDEO,
          Manifest.permission.READ_MEDIA_AUDIO
        ),
        REQUEST_CODE_PERMISSIONS
      )
    } else {
      // For Android 12 and below
      ActivityCompat.requestPermissions(
        this,
        arrayOf(
          Manifest.permission.READ_EXTERNAL_STORAGE,
          Manifest.permission.WRITE_EXTERNAL_STORAGE
        ),
        REQUEST_CODE_PERMISSIONS
      )
    }
  }

  override fun onDestroy() {
    multicastManager.release()
    super.onDestroy()
  }

  companion object {
    private const val REQUEST_CODE_PERMISSIONS = 100
  }
}
