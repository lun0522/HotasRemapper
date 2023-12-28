//
//  ContentView.swift
//  HotasRemapper
//
//  Created by Pujun Lun on 12/24/23.
//

import SwiftUI

struct ContentView: View {
  var text: String

  init(didGrantAccess: Bool) {
    text =
      didGrantAccess
      ? "Welcome to HOTAS Remapper!"
      : "You must grant input monitoring access and restart this app!"
  }

  var body: some View {
    VStack { Text(text) }.padding()
  }
}

#Preview {
  ContentView(didGrantAccess: true)
}
