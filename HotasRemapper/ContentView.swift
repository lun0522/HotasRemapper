//
//  ContentView.swift
//  HotasRemapper
//
//  Created by Pujun Lun on 12/24/23.
//

import SwiftUI

struct ContentView: View {
  @State private var connectionStatus = ObservableConnectionStatus.shared
  let didGrantAccess: Bool

  var body: some View {
    VStack {
      if didGrantAccess {
        Text(
          "Joystick connected: " + toString(connectionStatus.status.joystick))
        Text(
          "Throttle connected: " + toString(connectionStatus.status.throttle))
        Text(
          "Virtual device connected: "
            + toString(connectionStatus.status.virtual_device))
      } else {
        Text("You must grant input monitoring access and restart this app!")
      }
    }
    .padding()
  }
}

#Preview {
  ContentView(didGrantAccess: true)
}

private func toString(_ value: Bool) -> String {
  value ? "Yes" : "No"
}
