//
//  HotasRemapperApp.swift
//  HotasRemapper
//
//  Created by Pujun Lun on 12/24/23.
//

import IOKit
import SwiftUI

private let listenEventSettings: String =
  "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent"

private class AppDelegate: NSObject, NSApplicationDelegate {
  private static let cachedInputRemappingKey = "cachedInputRemapping"
  let didGrantAccess: Bool
  private var libHandle: UnsafeMutableRawPointer? = nil

  override init() {
    didGrantAccess = checkHIDAccess()
    super.init()
  }

  func applicationDidFinishLaunching(_ notification: Notification) {
    NSApp.activate()
    if didGrantAccess {
      libHandle = OpenLib(connectionStatusCallback)
      tryLoadCachedInputRemapping()
    } else {
      print("Not initializing due to lack of access")
    }
  }

  func applicationShouldTerminate(_ sender: NSApplication)
    -> NSApplication.TerminateReply
  {
    CloseLib(libHandle)
    return NSApplication.TerminateReply.terminateNow
  }

  func loadInputRemapping(from url: URL) {
    let inputRemapping: String
    do {
      inputRemapping = try String(contentsOf: url, encoding: .utf8)
    } catch {
      print("Failed to read input remapping file:", error)
      return
    }
    if inputRemapping.withCString({ inputRemappingPtr in
      LoadInputRemapping(libHandle, inputRemappingPtr)
    }) {
      print("Persisting input remapping file content")
      UserDefaults.standard.setValue(
        inputRemapping, forKey: AppDelegate.cachedInputRemappingKey)
    }
  }

  private func tryLoadCachedInputRemapping() {
    if let inputRemapping = UserDefaults.standard.string(
      forKey: AppDelegate.cachedInputRemappingKey)
    {
      print("Loading cached input remapping")
      let _ = inputRemapping.withCString({ inputRemappingPtr in
        LoadInputRemapping(libHandle, inputRemappingPtr)
      })
    } else {
      print("No input remapping cached")
    }
  }
}

@main
struct HotasRemapperApp: App {
  @NSApplicationDelegateAdaptor private var appDelegate: AppDelegate

  var body: some Scene {
    WindowGroup {
      ContentView(
        didGrantAccess: appDelegate.didGrantAccess,
        loadInputRemapping: { url in
          appDelegate.loadInputRemapping(from: url)
        })
    }
  }
}

private func checkHIDAccess() -> Bool {
  let accessType = IOHIDCheckAccess(kIOHIDRequestTypeListenEvent)
  print("Input monitoring access type:", toString(accessType))
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

extension Notification.Name {
  static let connectionStatusUpdate =
    Notification.Name("connectionStatusUpdate")
}

private func connectionStatusCallback(
  deviceType: DeviceType,
  isConnected: Bool
) {
  // Values must be published from the main thread.
  DispatchQueue.main.async {
    NotificationCenter.default.post(
      name: .connectionStatusUpdate,
      object: (deviceType, isConnected))
  }
}
