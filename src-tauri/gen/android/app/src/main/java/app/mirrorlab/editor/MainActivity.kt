package app.mirrorlab.editor

import android.os.Bundle
import android.view.View
import androidx.activity.SystemBarStyle
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    // Dark system bars with light icons, matching the editor.
    val dark = 0xFF0B0B0C.toInt()
    enableEdgeToEdge(
      statusBarStyle = SystemBarStyle.dark(dark),
      navigationBarStyle = SystemBarStyle.dark(dark),
    )
    super.onCreate(savedInstanceState)

    // Keep the web UI clear of the status bar, navigation bar, cutouts and keyboard.
    val content = findViewById<View>(android.R.id.content)
    ViewCompat.setOnApplyWindowInsetsListener(content) { view, insets ->
      val bars = insets.getInsets(
        WindowInsetsCompat.Type.systemBars() or
          WindowInsetsCompat.Type.displayCutout() or
          WindowInsetsCompat.Type.ime()
      )
      view.setPadding(bars.left, bars.top, bars.right, bars.bottom)
      WindowInsetsCompat.CONSUMED
    }
  }
}
