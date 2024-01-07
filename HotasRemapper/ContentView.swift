//
//  ContentView.swift
//  HotasRemapper
//
//  Created by Pujun Lun on 12/24/23.
//

import SwiftUI

struct ContentView: View {
  @State private var isImportingFile = false
  @State private var isJoystickConnected = false
  @State private var isThrottleConnected = false
  @State private var isVirtualDeviceConnected = false
  let didGrantAccess: Bool
  let loadInputMapping: (String) -> Void

  var body: some View {
    VStack {
      if didGrantAccess {
        Button("Load input mapping") {
          isImportingFile = true
        }
        .fileImporter(
          isPresented: $isImportingFile,
          allowedContentTypes: [.json],
          onCompletion: { result in
            switch result {
              case .success(let file):
                loadInputMapping(file.path())
              case .failure(let error):
                print(
                  "Failed to select input mapping file",
                  error.localizedDescription)
            }
          })
        Text("Joystick connected: " + toString(isJoystickConnected))
        Text("Throttle connected: " + toString(isThrottleConnected))
        Text("Virtual device connected: " + toString(isVirtualDeviceConnected))
      } else {
        Text("You must grant input monitoring access and restart this app!")
      }
    }
    .onReceive(
      NotificationCenter.default.publisher(for: .connectionStatusUpdate),
      perform: { notification in
        if let connectionStatus = notification.object as? (DeviceType, Bool) {
          let (deviceType, isConnected) = connectionStatus
          switch deviceType {
            case kJoystick:
              isJoystickConnected = isConnected
            case kThrottle:
              isThrottleConnected = isConnected
            case kVirtualDevice:
              isVirtualDeviceConnected = isConnected
            default:
              print("Unknown device type:", deviceType)
          }
        }
      }
    )
    .padding()
  }
}

#Preview {
  ContentView(didGrantAccess: true, loadInputMapping: { _ in })
}

private func toString(_ value: Bool) -> String {
  value ? "Yes" : "No"
}
