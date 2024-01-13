import IOBluetooth

public typealias ConnectionStatusCallback = @convention(c) (Bool) -> Void

@_cdecl("OpenBluetoothLib")
public func openBluetoothLib(
  virtualDeviceCallback: ConnectionStatusCallback,
  rfcommChannelCallback: ConnectionStatusCallback
) {
  BluetoothManager.shared.start(
    with: virtualDeviceCallback, and: rfcommChannelCallback)
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

  private var virtualDeviceCallback: ConnectionStatusCallback?
  private var rfcommChannelCallback: ConnectionStatusCallback?
  private var isRunning = false
  private var isVirtualDeviceConnected = false {
    didSet {
      virtualDeviceCallback?(isVirtualDeviceConnected)
    }
  }
  // rfcommChannel will have value no matter whether the channel is successfully
  // opened or not, so don't check the status by checking if it has value.
  private var rfcommChannel: IOBluetoothRFCOMMChannel?
  private var isRFCOMMChannelConnected = false {
    didSet {
      rfcommChannelCallback?(isRFCOMMChannelConnected)
    }
  }

  func start(
    with virtualDeviceCallback: ConnectionStatusCallback,
    and rfcommChannelCallback: ConnectionStatusCallback
  ) {
    print("Starting Bluetooth manager")
    isRunning = true
    self.virtualDeviceCallback = virtualDeviceCallback
    self.rfcommChannelCallback = rfcommChannelCallback
    IOBluetoothDevice.register(
      forConnectNotifications: self,
      selector: #selector(didConnect(notification:fromDevice:)))
  }

  func send(buffer: UnsafePointer<CChar>, length: Int32) {
    if isRFCOMMChannelConnected, let channel = rfcommChannel {
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
    if isRFCOMMChannelConnected, let channel = rfcommChannel {
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
      &rfcommChannel,
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
      self.rfcommChannel = rfcommChannel
      self.isRFCOMMChannelConnected = true
    } else {
      print("Failed to open RFCOMM channel:", error)
    }
  }

  func rfcommChannelClosed(_ rfcommChannel: IOBluetoothRFCOMMChannel!) {
    if rfcommChannel == self.rfcommChannel {
      print("RFCOMM channel closed")
      self.rfcommChannel = nil
      self.isRFCOMMChannelConnected = false
    }
  }
}

private func toString(_ device: IOBluetoothDevice) -> String {
  """
  {device name: "\(device.name ?? "Unknown name")", \
  address: "\(device.addressString ?? "Unknown address")"}
  """
}
