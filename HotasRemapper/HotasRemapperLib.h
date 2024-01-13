//
//  HotasRemapperLib.h
//  HotasRemapper
//
//  Created by Pujun Lun on 12/24/23.
//

#ifndef HotasRemapperLib_h
#define HotasRemapperLib_h

#include <stdbool.h>
#include <stdio.h>

enum ConnectionType {
  kJoystick = 0,
  kThrottle = 1,
  kVirtualDevice = 2,
  kRFCOMMChannel = 3,
};

void* OpenLib(void (*connection_status_callback)(
    enum ConnectionType connection_type, bool is_connected));
bool LoadInputRemapping(void* lib_handle, const char* input_remapping_ptr);
void CloseLib(void* lib_handle);

#endif /* HotasRemapperLib_h */
