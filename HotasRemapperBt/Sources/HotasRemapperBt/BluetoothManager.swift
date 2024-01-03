import IOBluetooth

public typealias ConnectionStatusCallback = @convention(c) (Bool) -> Void

@_cdecl("OpenBluetoothLib")
public func openBluetoothLib(callback: ConnectionStatusCallback) {
  BluetoothManager.shared.start(withCallback: callback)
}

@_cdecl("SendDataViaBluetooth")
public func sendDataViaBluetooth(buffer: UnsafePointer<CChar>, length: Int32) {
  BluetoothManager.shared.send(buffer: buffer, length: length)
}

@_cdecl("CloseBluetoothLib")
public func closeBluetoothLib() {
  BluetoothManager.shared.stop()
}

class BluetoothManager: NSObject, IOBluetoothRFCOMMChannelDelegate {
  static let shared = BluetoothManager()

  var isRunning = false
  var isVirtualDeviceConnected = false {
    didSet {
      connectionStatusCallback?(isVirtualDeviceConnected)
    }
  }
  var connectedChannel: IOBluetoothRFCOMMChannel?
  var connectionStatusCallback: ConnectionStatusCallback?

  func start(withCallback callback: ConnectionStatusCallback) {
    print("Starting Bluetooth manager")
    isRunning = true
    connectionStatusCallback = callback
    IOBluetoothDevice.register(
      forConnectNotifications: self,
      selector: #selector(didConnect(notification:fromDevice:)))
  }

  func send(buffer: UnsafePointer<CChar>, length: Int32) {
    if let channel = connectedChannel {
      let ret = channel.writeSync(
        UnsafeMutableRawPointer(mutating: buffer), length: UInt16(length))
      if ret != kIOReturnSuccess {
        print("Failed to write to RFCOMM channel:", ret)
      }
    }
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
    if isVirtualDeviceConnected {
      return
    }
    isVirtualDeviceConnected = true
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
    isVirtualDeviceConnected = false
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
