package com.tepubeditor.app

import android.content.res.Configuration
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.os.Build
import android.os.Bundle
import android.view.WindowInsets
import android.view.WindowManager
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.content.ContextCompat
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import androidx.core.view.WindowCompat
import java.util.Locale

class MainActivity : TauriActivity() {
  private var webView: WebView? = null

  @Volatile
  private var safeAreaInsetsJson =
    "{\"top\":24,\"bottom\":0,\"left\":0,\"right\":0}"

  override fun onCreate(savedInstanceState: Bundle?) {
    installSplashScreen()
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    applySystemBars(resources.configuration.uiMode and
      android.content.res.Configuration.UI_MODE_NIGHT_MASK ==
      android.content.res.Configuration.UI_MODE_NIGHT_YES)
  }

  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    this.webView = webView
    webView.setBackgroundColor(
      ContextCompat.getColor(this, R.color.splash_background),
    )
    webView.addJavascriptInterface(SystemBarsBridge(), "TepubAndroid")
  }

  override fun onConfigurationChanged(newConfig: Configuration) {
    super.onConfigurationChanged(newConfig)
    applySystemBars(
      newConfig.uiMode and Configuration.UI_MODE_NIGHT_MASK ==
        Configuration.UI_MODE_NIGHT_YES,
    )
  }

  private fun applySystemBars(dark: Boolean) {
    runOnUiThread {
      WindowCompat.setDecorFitsSystemWindows(window, false)
      window.setBackgroundDrawable(
        ColorDrawable(ContextCompat.getColor(this, R.color.splash_background)),
      )
      window.setLayout(
        WindowManager.LayoutParams.MATCH_PARENT,
        WindowManager.LayoutParams.MATCH_PARENT,
      )
      window.clearFlags(
        WindowManager.LayoutParams.FLAG_TRANSLUCENT_STATUS or
          WindowManager.LayoutParams.FLAG_TRANSLUCENT_NAVIGATION,
      )
      window.addFlags(
        WindowManager.LayoutParams.FLAG_DRAWS_SYSTEM_BAR_BACKGROUNDS or
          WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN,
      )
      @Suppress("DEPRECATION")
      window.statusBarColor = Color.TRANSPARENT
      @Suppress("DEPRECATION")
      window.navigationBarColor = Color.TRANSPARENT
      if (Build.VERSION.SDK_INT >= 28) {
        window.navigationBarDividerColor = Color.TRANSPARENT
        window.attributes = window.attributes.apply {
          layoutInDisplayCutoutMode =
            WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES
        }
      }
      if (Build.VERSION.SDK_INT >= 29) {
        window.isStatusBarContrastEnforced = false
        window.isNavigationBarContrastEnforced = false
      }
      WindowCompat.getInsetsController(window, window.decorView).apply {
        isAppearanceLightStatusBars = !dark
        isAppearanceLightNavigationBars = !dark
      }
      // applySystemBars 可能在首帧布局前运行；等 Insets 稳定后再缓存在
      // 原生侧，JS bridge 是同步调用，不能从 JavaBridge 线程直接碰 Window。
      window.decorView.post {
        safeAreaInsetsJson = readSafeAreaInsetsJson()
        webView?.post {
          webView?.evaluateJavascript(
            "window.__tepubApplyNativeSafeArea?.();",
            null,
          )
        }
      }
    }
  }

  private fun readSafeAreaInsetsJson(): String {
    val density = resources.displayMetrics.density.coerceAtLeast(1f)
    var top = 0
    var bottom = 0
    var left = 0
    var right = 0

    if (Build.VERSION.SDK_INT >= 30) {
      val insets = window.decorView.rootWindowInsets?.getInsets(
        WindowInsets.Type.systemBars() or WindowInsets.Type.displayCutout(),
      )
      if (insets != null) {
        top = insets.top
        bottom = insets.bottom
        left = insets.left
        right = insets.right
      }
    } else {
      @Suppress("DEPRECATION")
      val insets = window.decorView.rootWindowInsets
      if (insets != null) {
        @Suppress("DEPRECATION")
        top = insets.systemWindowInsetTop
        @Suppress("DEPRECATION")
        bottom = insets.systemWindowInsetBottom
        @Suppress("DEPRECATION")
        left = insets.systemWindowInsetLeft
        @Suppress("DEPRECATION")
        right = insets.systemWindowInsetRight
      }
    }

    fun css(px: Int) =
      String.format(Locale.US, "%.2f", px / density)
        .trimEnd('0')
        .trimEnd('.')
    return "{\"top\":${css(top)},\"bottom\":${css(bottom)}," +
      "\"left\":${css(left)},\"right\":${css(right)}}"
  }

  private inner class SystemBarsBridge {
    @JavascriptInterface
    fun setDarkMode(dark: Boolean) {
      applySystemBars(dark)
    }

    @JavascriptInterface
    fun getSafeAreaInsets(): String = safeAreaInsetsJson
  }
}
