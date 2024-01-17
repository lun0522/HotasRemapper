//
//  ContentView.swift
//  HotasRemapper
//
//  Created by Pujun Lun on 12/24/23.
//

import SwiftUI

struct ContentView: View {
  @State private var isImportingSettingsFile = false
  @State private var isImportingInputRemappingFile = false
  @State private var isJoystickConnected = false
  @State private var isThrottleConnected = false
  @State private var isVirtualDeviceConnected = false
  @State private var isRFCOMMChannelConnected = false

  let didGrantAccess: Bool
  let loadSettings: (URL) -> Void
  let loadInputRemapping: (URL) -> Void

  var body: some View {
    VStack {
      if didGrantAccess {
        Button("Load settings (requires restart)") {
          isImportingSettingsFile = true
        }
        .fileImporter(
          isPresented: $isImportingSettingsFile,
          allowedContentTypes: [.item],
          onCompletion: { result in
            switch result {
              case .success(let url):
                loadSettings(url)
              case .failure(let error):
                print(
                  "Failed to select settings file",
                  error.localizedDescription)
            }
          })
        Button("Load input remapping") {
          isImportingInputRemappingFile = true
        }
        .fileImporter(
          isPresented: $isImportingInputRemappingFile,
          allowedContentTypes: [.item],
          onCompletion: { result in
            switch result {
              case .success(let url):
                loadInputRemapping(url)
              case .failure(let error):
                print(
                  "Failed to select input remapping file",
                  error.localizedDescription)
            }
          })
        Text("Joystick connected: " + toString(isJoystickConnected))
        Text("Throttle connected: " + toString(isThrottleConnected))
        Text("Virtual device connected: " + toString(isVirtualDeviceConnected))
        Text("RFCOMM channel connected: " + toString(isRFCOMMChannelConnected))
      } else {
        Text("You must grant input monitoring access and restart this app!")
      }
    }
    .onReceive(
      NotificationCenter.default.publisher(for: .connectionStatusUpdate),
      perform: { notification in
        if let connectionStatus = notification.object as? (ConnectionType, Bool)
        {
          let (connectionType, isConnected) = connectionStatus
          switch connectionType {
            case kJoystick:
              isJoystickConnected = isConnected
            case kThrottle:
              isThrottleConnected = isConnected
            case kVirtualDevice:
              isVirtualDeviceConnected = isConnected
            case kRFCOMMChannel:
              isRFCOMMChannelConnected = isConnected
            default:
              print("Unknown connection type:", connectionType)
          }
        }
      }
    )
    .padding()
  }
}

#Preview {
  ContentView(
    didGrantAccess: true,
    loadSettings: { _ in },
    loadInputRemapping: { _ in })
}

private func toString(_ value: Bool) -> String {
  value ? "Yes" : "No"
}
