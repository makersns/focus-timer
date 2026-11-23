# The Tauri plugin is loaded reflectively; JNI names must survive release shrinking.
-keep class com.makersns.focus.FocusPlugin { *; }
-keep class com.makersns.focus.FocusNative { *; }
-keepclasseswithmembernames,includedescriptorclasses class * {
    native <methods>;
}
