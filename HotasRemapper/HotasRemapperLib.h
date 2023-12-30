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

struct ConnectionStatus {
  bool joystick;
  bool throttle;
  bool virtual_device;
};

void* OpenLib(
    void (*connection_status_callback)(const struct ConnectionStatus*));
void CloseLib(void* handle);

#endif /* HotasRemapperLib_h */
