#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <unistd.h>
#include <math.h>


#include "freertos/FreeRTOS.h"
#include "ffi.h"


#include "freertos/task.h"
#include "freertos/semphr.h"
#include "esp_timer.h"
#include "driver/gpio.h"

#define STACK_SIZE 4096
#define DELAY 1000
#define COUNTER_MAX 10

SemaphoreHandle_t s_sr04;
SemaphoreHandle_t s_blink;
SemaphoreHandle_t s_button;
SemaphoreHandle_t s_scaler;

QueueHandle_t temp_queue;
QueueHandle_t dist_queue;

uint8_t scaler = 1;
double temp = 0;
double dist = 0;

long long int start_rng = 0;


uint8_t scaler_read(){
    uint8_t scaler_copy = 0;
    xSemaphoreTake(s_scaler, portMAX_DELAY);
    scaler_copy = scaler;
    xSemaphoreGive(s_scaler);
    return scaler_copy;
}

void poll_bmp180(){
    printf("name,waitingTime,runningTime,rngTime\n");
    while(true) {
        long long int start = esp_timer_get_time();
        start_rng = start;
        // 1. Sample the BMP180 sensor once through the related FFI function.
        temp = ffi_bmp180();
        // 2. Unlock the blink task to visually indicate the sampling.
        xSemaphoreGive(s_blink);
        // 3. Send the temperature value to the sha256_task task.
        xQueueSend(temp_queue, &temp, portMAX_DELAY);
        // 4. Read the frequency scaling factor.
        const TickType_t task_delay = DELAY / (pow(2, scaler_read()-1) * portTICK_PERIOD_MS);  
        // 5. Unlock the poll_sr04 task.
        xSemaphoreGive(s_sr04);
        long long int start_waiting = esp_timer_get_time();
        // 6. Wait for the period defined by the scaling factor
        vTaskDelay(task_delay);

        long long int stop = esp_timer_get_time();        
        printf("%s,%lld,%lld,0\n", "BMP180", stop-start_waiting, start_waiting-start);        
    }
}

void poll_sr04(){
    // This task polls the SR04 distance sensor once through the related FFI function, then sends the value to the sha256_task task
    while(true){
        long long int start_waiting = esp_timer_get_time();
        xSemaphoreTake(s_sr04, portMAX_DELAY);
        long long int start = esp_timer_get_time();
        //1. Sample the SR04
        dist = ffi_sr04();
        // 2. Send value to sha256 task
        xQueueSend(dist_queue, &dist, portMAX_DELAY);
        long long int stop = esp_timer_get_time();
        printf("%s,%lld,%lld,0\n", "SR04", start-start_waiting, stop-start);
    }
}

void sha256_task(){
    while(true){
        long long int start_waiting = esp_timer_get_time();
        // add semaphore count here ?
        // This task collects one temperature value and one distance value, then performs a XOR
        double temperature = 0;
        double distance = 0;
        if(xQueueReceive(temp_queue, &temperature, portMAX_DELAY) == pdTRUE && xQueueReceive(dist_queue, &distance, portMAX_DELAY) == pdTRUE){
            long long int start = esp_timer_get_time();

            double xor = (double) (*(unsigned long long *)&temperature ^ *(unsigned long long *)&distance);
            // operaton between both values and finally computes the SHA256 hash of the XORed value. The final
            Array sha = ffi_sha256(xor);
            long long int stop = esp_timer_get_time();
            // random value is printed on the UART, along with the temperature and distance values
            // printf("SHA: [");
            // for(int i = 0; i<32; i++) {
            //     if (i<31) { printf("%d, ", sha._0[i]); }
            //     else { printf("%d", sha._0[31]); }
            // }
            // printf("]\n");
            // printf("Temperature: %.1f°C\tDistance: %.2fm\n\n", temp, dist);
            printf("%s,%lld,%lld,%lld\n", "SHA256", start-start_waiting, stop-start, stop-start_rng);
            start_rng = 0;
        }
    }
}

void blink(){
    while(true){
        // This task handles the embedded LED through the dedicated FFI function.
        xSemaphoreTake(s_blink, portMAX_DELAY);
        ffi_blink();
    }
}

void button(){
    while(true){
        // This task, unlocked by the button interrupt,  
        xSemaphoreTake(s_button, portMAX_DELAY);
        // Increments the scaler by one, wrapping it around in case of overload.
        xSemaphoreTake(s_scaler, portMAX_DELAY);
        scaler = scaler % 7;
        scaler += 1;
        xSemaphoreGive(s_scaler);
        // It then displays the binary scaler value on the three-LED stand.
        ffi_leds(scaler_read());
    }
}



void IRAM_ATTR button_isr(void *arg) {
    xSemaphoreGiveFromISR(s_button, NULL);
}


/**
 * Main function called by the ESP-IDF SDK.
 */

int app_main(void) {

    // printf("Hello from ESP-IDF.\n");    
    ffi_setup();

    // ESP-IDF to setup the button interrupt rather than the bare-metal HAL    
    gpio_config_t io_conf;
    io_conf.intr_type = GPIO_INTR_POSEDGE;
    io_conf.mode = GPIO_MODE_INPUT;
    io_conf.pin_bit_mask = 1ULL << GPIO_NUM_0;
    gpio_config(&io_conf);
    gpio_install_isr_service(false);
    gpio_isr_handler_add(GPIO_NUM_0, &button_isr, NULL);

    s_scaler = xSemaphoreCreateBinary();
    if (s_scaler == NULL)
    {
        printf("ERROR: s_scaler\n");
        return 1;
    }
    xSemaphoreGive(s_scaler);

    s_sr04 = xSemaphoreCreateBinary();
    if (s_sr04 == NULL)
    {
        printf("ERROR: s_sr04\n");
        return 1;
    }

    s_blink = xSemaphoreCreateBinary();
    if (s_blink == NULL)
    {
        printf("ERROR: s_blink\n");
        return 1;
    }

    s_button = xSemaphoreCreateBinary();
    if (s_button == NULL)
    {
        printf("ERROR: s_button\n");
        return 1;
    }

    temp_queue = xQueueCreate(1, sizeof(double));
    if (temp_queue == NULL)
    {
        printf("ERROR: temp_queue\n");
        return 1;
    }

    dist_queue = xQueueCreate(1, sizeof(double));
    if (dist_queue == NULL)
    {
        printf("ERROR: dist_queue\n");
        return 1;
    }

    ffi_leds(scaler_read());

    // docs.espressif.com/projects/esp-idf/en/v4.3/esp32/api-reference/system/freertos.html#task-api
    xTaskCreatePinnedToCore(blink, "blink", STACK_SIZE, NULL, 20|portPRIVILEGE_BIT, NULL, 1);
    xTaskCreatePinnedToCore(sha256_task, "sha256", STACK_SIZE, NULL, 5|portPRIVILEGE_BIT, NULL, 1);
    xTaskCreatePinnedToCore(poll_sr04, "poll_sr04", STACK_SIZE, NULL, 10|portPRIVILEGE_BIT, NULL, 0);
    xTaskCreatePinnedToCore(button, "button", STACK_SIZE, NULL, 100|portPRIVILEGE_BIT, NULL, 1);
    xTaskCreatePinnedToCore(poll_bmp180, "poll_bmp180", STACK_SIZE, NULL, 10|portPRIVILEGE_BIT, NULL, 0);

    return 0;
}
