//
//  HotasRemapperApp.swift
//  HotasRemapper
//
//  Created by Pujun Lun on 12/24/23.
//

import IOKit
import SwiftUI

let listenEventSettings: String =
  "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent"

private class AppDelegate: NSObject, NSApplicationDelegate {
  var didGrantAccess: Bool
  var libHandle: UnsafeMutableRawPointer? = nil

  override init() {
    didGrantAccess = checkHIDAccess()
    super.init()
  }

  func applicationDidFinishLaunching(_ notification: Notification) {
    NSApp.activate()
    if (didGrantAccess) {
      libHandle = OpenLib()
    }
  }

  func applicationShouldTerminate(_ sender: NSApplication)
    -> NSApplication.TerminateReply
  {
    if (libHandle != nil) {
      CloseLib(libHandle)
    }
    return NSApplication.TerminateReply.terminateNow
  }
}

@main
struct HotasRemapperApp: App {
  @NSApplicationDelegateAdaptor private var appDelegate: AppDelegate

  var body: some Scene {
    WindowGroup {
      ContentView(didGrantAccess: appDelegate.didGrantAccess)
    }
  }
}

private func checkHIDAccess() -> Bool {
  let accessType = IOHIDCheckAccess(kIOHIDRequestTypeListenEvent)
  print("Current access type:", toString(accessType))
  switch accessType {
    case kIOHIDAccessTypeGranted:
      return true;

    // Access is either denied or granted to an old binary of this app.
    case kIOHIDAccessTypeDenied:
      NSWorkspace.shared.open(URL.init(string: listenEventSettings)!)
      return false;

    // Never requested the access for this app before.
    default:
      IOHIDRequestAccess(kIOHIDRequestTypeListenEvent);
      return false;
  }
}

private func toString(_ type: IOHIDAccessType) -> String {
  switch type {
    case kIOHIDAccessTypeGranted: "Granted"
    case kIOHIDAccessTypeDenied: "Denied"
    default: "Unknown"
  }
}
