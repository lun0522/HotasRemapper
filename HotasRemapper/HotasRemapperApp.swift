//
//  HotasRemapperApp.swift
//  HotasRemapper
//
//  Created by Pujun Lun on 12/24/23.
//

import SwiftUI

private class AppDelegate: NSObject, NSApplicationDelegate {
  func applicationDidFinishLaunching(_ notification: Notification) {
    InitLib()
    NSApp.activate()
  }

  func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication)
    -> Bool
  {
    true
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
