package com.proanima.googleauth

import android.accounts.Account
import android.app.Activity
import androidx.activity.result.ActivityResult
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import com.google.android.gms.auth.GoogleAuthUtil
import com.google.android.gms.auth.UserRecoverableAuthException
import com.google.android.gms.auth.api.signin.GoogleSignIn
import com.google.android.gms.auth.api.signin.GoogleSignInAccount
import com.google.android.gms.auth.api.signin.GoogleSignInOptions
import com.google.android.gms.common.api.ApiException
import com.google.android.gms.common.api.Scope
import kotlin.concurrent.thread

@InvokeArg
class TokenArgs {
  var scope: String? = null
}

@TauriPlugin
class GoogleAuthPlugin(private val activity: Activity) : Plugin(activity) {
  private val prefs by lazy {
    activity.getSharedPreferences("focusnook_google_auth", Activity.MODE_PRIVATE)
  }

  @Command
  fun connect(invoke: Invoke) {
    val args = invoke.parseArgs(TokenArgs::class.java)
    val scope = args.scope
    if (scope.isNullOrBlank()) {
      invoke.reject("scope is required")
      return
    }
    prefs.edit().putString(KEY_LAST_SCOPE, scope).apply()

    val options = GoogleSignInOptions.Builder(GoogleSignInOptions.DEFAULT_SIGN_IN)
      .requestEmail()
      .requestScopes(Scope(scope))
      .build()
    startActivityForResult(
      invoke,
      GoogleSignIn.getClient(activity, options).signInIntent,
      "handleSignInResult",
    )
  }

  @ActivityCallback
  fun handleSignInResult(invoke: Invoke, result: ActivityResult) {
    if (result.resultCode != Activity.RESULT_OK) {
      invoke.reject("Google account selection was cancelled")
      return
    }

    try {
      val account = GoogleSignIn.getSignedInAccountFromIntent(result.data)
        .getResult(ApiException::class.java)
      if (account == null) {
        invoke.reject("Google account was not returned")
        return
      }
      saveAccount(account)
      resolveToken(invoke, account.email, prefs.getString(KEY_LAST_SCOPE, DRIVE_APPDATA_SCOPE))
    } catch (error: ApiException) {
      invoke.reject("Google sign-in failed: ${error.statusCode}")
    }
  }

  @Command
  fun accessToken(invoke: Invoke) {
    val args = invoke.parseArgs(TokenArgs::class.java)
    val scope = args.scope
    if (scope.isNullOrBlank()) {
      invoke.reject("scope is required")
      return
    }

    val email = prefs.getString(KEY_EMAIL, null)
    if (email.isNullOrBlank()) {
      invoke.reject("Google account is not connected")
      return
    }
    resolveToken(invoke, email, scope)
  }

  @Command
  fun isConnected(invoke: Invoke) {
    val email = prefs.getString(KEY_EMAIL, null)
    val ret = JSObject()
    ret.put("connected", !email.isNullOrBlank())
    ret.put("email", email)
    invoke.resolve(ret)
  }

  @Command
  fun disconnect(invoke: Invoke) {
    val options = GoogleSignInOptions.Builder(GoogleSignInOptions.DEFAULT_SIGN_IN).build()
    GoogleSignIn.getClient(activity, options).signOut().addOnCompleteListener {
      prefs.edit().clear().apply()
      invoke.resolve()
    }
  }

  private fun saveAccount(account: GoogleSignInAccount) {
    prefs.edit()
      .putString(KEY_EMAIL, account.email)
      .apply()
  }

  private fun resolveToken(invoke: Invoke, email: String?, scope: String?) {
    if (email.isNullOrBlank() || scope.isNullOrBlank()) {
      invoke.reject("Google account or scope is missing")
      return
    }

    prefs.edit().putString(KEY_LAST_SCOPE, scope).apply()
    thread(name = "focusnook-google-token") {
      try {
        val token = GoogleAuthUtil.getToken(
          activity,
          Account(email, GoogleAuthUtil.GOOGLE_ACCOUNT_TYPE),
          "oauth2:$scope",
        )
        val ret = JSObject()
        ret.put("accessToken", token)
        ret.put("email", email)
        activity.runOnUiThread { invoke.resolve(ret) }
      } catch (recoverable: UserRecoverableAuthException) {
        activity.runOnUiThread {
          val intent = recoverable.intent
          if (intent == null) {
            invoke.reject("Google authorization requires user action but no recovery intent was returned")
          } else {
            startActivityForResult(invoke, intent, "handleRecoverableAuthResult")
          }
        }
      } catch (error: Exception) {
        activity.runOnUiThread {
          invoke.reject("Google token request failed: ${error.message ?: error.javaClass.simpleName}")
        }
      }
    }
  }

  @ActivityCallback
  fun handleRecoverableAuthResult(invoke: Invoke, result: ActivityResult) {
    if (result.resultCode != Activity.RESULT_OK) {
      invoke.reject("Google authorization was cancelled")
      return
    }
    val email = prefs.getString(KEY_EMAIL, null)
    val scope = prefs.getString(KEY_LAST_SCOPE, DRIVE_APPDATA_SCOPE)
    resolveToken(invoke, email, scope)
  }

  companion object {
    private const val KEY_EMAIL = "email"
    private const val KEY_LAST_SCOPE = "last_scope"
    private const val DRIVE_APPDATA_SCOPE = "https://www.googleapis.com/auth/drive.appdata"
  }
}
