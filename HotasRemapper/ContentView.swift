//
//  ContentView.swift
//  HotasRemapper
//
//  Created by Pujun Lun on 12/24/23.
//

import SwiftUI

struct ContentView: View {
  var body: some View {
    VStack {
      Text("Welcome to HOTAS Remapper!")
      Button(
        action: {
          PrintProjectName()
        },
        label: {
          Text("Log project name")
        }
      )
    }
    .padding()
  }
}

#Preview {
  ContentView()
}
