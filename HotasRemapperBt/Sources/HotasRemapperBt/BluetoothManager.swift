import IOBluetooth

@_cdecl("open_bluetooth_lib")
public func openBluetoothLib() {
  BluetoothManager.shared.start()
}

@_cdecl("close_bluetooth_lib")
public func closeBluetoothLib() {
  BluetoothManager.shared.stop()
}

class BluetoothManager: NSObject, IOBluetoothRFCOMMChannelDelegate {
  static let shared = BluetoothManager()

  var connectedChannel: IOBluetoothRFCOMMChannel?
  var isRunning = false
  var isVirtualDeviceFound = false

  func start() {
    print("Starting Bluetooth manager")
    isRunning = true
    IOBluetoothDevice.register(
      forConnectNotifications: self,
      selector: #selector(didConnect(notification:fromDevice:)))
  }

  func stop() {
    print("Stopping Bluetooth manager")
    isRunning = false
    if let channel = connectedChannel {
      print("Closing RFCOMM channel")
      channel.close()
    }
  }

  @objc private func didConnect(
    notification: IOBluetoothUserNotification,
    fromDevice: IOBluetoothDevice
  ) {
    if !isRunning {
      notification.unregister()
      return
    }

    let deviceInfo = toString(fromDevice);
    if fromDevice.addressString != "b8-27-eb-c7-5b-1d" {
      print("Ignoring Bluetooth device:", deviceInfo)
      return
    }

    print("Found virtual device via Bluetooth:", deviceInfo)
    // We may get notified more than once.
    if isVirtualDeviceFound {
      return
    }
    isVirtualDeviceFound = true
    fromDevice.register(
      forDisconnectNotification: self,
      selector: #selector(didDisconnect(notification:fromDevice:)))

    print("Opening RFCOMM channel")
    fromDevice.openRFCOMMChannelAsync(
      &connectedChannel,
      withChannelID: 1,
      delegate: self)
  }

  @objc private func didDisconnect(
    notification: IOBluetoothUserNotification,
    fromDevice: IOBluetoothDevice
  ) {
    print("Virtual device disconnected")
    isVirtualDeviceFound = false
    if !isRunning {
      notification.unregister()
    }
  }

  // MARK: - IOBluetoothRFCOMMChannelDelegate

  func rfcommChannelOpenComplete(
    _ rfcommChannel: IOBluetoothRFCOMMChannel!,
    status error: IOReturn
  ) {
    if error == kIOReturnSuccess {
      print("Opened RFCOMM channel")
      connectedChannel = rfcommChannel
    } else {
      print("Failed to open RFCOMM channel:", error)
    }
  }

  func rfcommChannelClosed(_ rfcommChannel: IOBluetoothRFCOMMChannel!) {
    if rfcommChannel == connectedChannel {
      print("RFCOMM channel closed")
      connectedChannel = nil
    }
  }
}

private func toString(_ device: IOBluetoothDevice) -> String {
  """
  {device name: "\(device.name ?? "Unknown name")", \
  address: "\(device.addressString ?? "Unknown address")"}
  """
}
