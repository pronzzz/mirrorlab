package app.mirrorlab.editor

import android.app.Activity
import android.content.ContentValues
import android.os.Environment
import android.provider.MediaStore
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.File

@InvokeArg
class SaveArgs {
  lateinit var path: String
  lateinit var name: String
  lateinit var mime: String
}

/**
 * Copies an exported file into shared storage through MediaStore:
 * photos go to Pictures/MirrorLab (visible in the gallery), other files to Downloads/MirrorLab.
 * Needs no storage permission on Android 10+.
 */
@TauriPlugin
class GalleryPlugin(private val activity: Activity) : Plugin(activity) {
  @Command
  fun saveToGallery(invoke: Invoke) {
    try {
      val args = invoke.parseArgs(SaveArgs::class.java)
      val source = File(args.path)
      val isImage = args.mime.startsWith("image/")
      val resolver = activity.contentResolver
      val collection =
        if (isImage) MediaStore.Images.Media.EXTERNAL_CONTENT_URI else MediaStore.Downloads.EXTERNAL_CONTENT_URI
      val folder = (if (isImage) Environment.DIRECTORY_PICTURES else Environment.DIRECTORY_DOWNLOADS) + "/MirrorLab"
      val values = ContentValues().apply {
        put(MediaStore.MediaColumns.DISPLAY_NAME, args.name)
        put(MediaStore.MediaColumns.MIME_TYPE, args.mime)
        put(MediaStore.MediaColumns.RELATIVE_PATH, folder)
        put(MediaStore.MediaColumns.IS_PENDING, 1)
      }
      val uri = resolver.insert(collection, values) ?: throw IllegalStateException("Could not create the file")
      try {
        resolver.openOutputStream(uri)?.use { out -> source.inputStream().use { it.copyTo(out) } }
          ?: throw IllegalStateException("Could not open the file for writing")
        values.clear()
        values.put(MediaStore.MediaColumns.IS_PENDING, 0)
        resolver.update(uri, values, null, null)
      } catch (e: Exception) {
        resolver.delete(uri, null, null)
        throw e
      }
      source.delete()
      val ret = JSObject()
      ret.put("location", folder)
      invoke.resolve(ret)
    } catch (e: Exception) {
      invoke.reject(e.message ?: "Could not save the file")
    }
  }
}
