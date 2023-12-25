//
//  HotasRemapperApp.swift
//  HotasRemapper
//
//  Created by Pujun Lun on 12/24/23.
//

import SwiftUI

private class AppDelegate: NSObject, NSApplicationDelegate {
  var libHandle: UnsafeMutableRawPointer? = nil

  func applicationDidFinishLaunching(_ notification: Notification) {
    NSApp.activate()
    libHandle = OpenLib()
  }

  func applicationShouldTerminate(_ sender: NSApplication)
    -> NSApplication.TerminateReply
  {
    CloseLib(libHandle)
    return NSApplication.TerminateReply.terminateNow
  }
}

@main
struct HotasRemapperApp: App {
  @NSApplicationDelegateAdaptor private var appDelegate: AppDelegate

  var body: some Scene {
    WindowGroup {
      ContentView()
    }
  }
}
