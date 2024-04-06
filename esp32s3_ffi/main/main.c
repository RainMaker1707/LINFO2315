#include <stdint.h>
#include <stdio.h>
#include <stdbool.h>
#include <unistd.h>

#include "freertos/FreeRTOS.h"
#include "ffi.h"

#define STACK_SIZE 4096


void poll_bmp180(){
    // 1. Sample the BMP180 sensor once through the related FFI function.
    // 2. Unlock the blink task to visually indicate the sampling.
    // 3. Send the temperature value to the sha256_task task.
    // 4. Read the frequency scaling factor.
    // 5. Unlock the poll_sr04 task.
    // 6. Wait for the period defined by the scaling factor
}

void poll_sr04(){
    // This task polls the SR04 distance sensor once through the related FFI function, then sends the value to the sha256_task task
}

void sha256_task(){
    // This task collects one temperature value and one distance value, then performs a XOR
    // operaton between both values and finally computes the SHA256 hash of the XORed value. The final
    // random value is printed on the UART, along with the temperature and distance values
}

void blink(){
    // This task handles the embedded LED through the dedicated FFI function.
}

void button(){
    // This task, unlocked by the button interrupt, increments the scaler by one, wrapping it around
    // in case of overload. It then displays the binary scaler value on the three-LED stand. You must use the
    // ESP-IDF to setup the button interrupt rather than the bare-metal HAL
}

/**
 * Main function called by the ESP-IDF SDK.
 */
void app_main(void) {

    printf("Hello from ESP-IDF.\n");

    ffi_setup();
    
    //int scaler = 0;
    while(true){
        ffi_blink();
        // ffi_leds(scaler++);
        // if (scaler > 7) {
        //     scaler = 0;
        // }
        double dist = ffi_sr04();
        printf("SR04: %.2fm\n", dist);
        double temp = ffi_bmp180();
        printf("Temperature %.1f°C\n", temp);
        sleep(1);
    }
}
