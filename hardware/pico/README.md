# BlackGPIB Pico

![Photo](/photo/pico-rev1.jpg)

## Components

| Part                     | Value                     | Package                  | Quantity | Comment                                                              |
|--------------------------|---------------------------|--------------------------|----------|----------------------------------------------------------------------|
| GPIB Connector Female    | -                         | 90 Degree                | 1        |                                                                      |
| SN75160                  | -                         | SMD or DIP               | 1        |                                                                      |
| SN75161 / SN75162        | -                         | SMD or DIP               | 1        |                                                                      |
| Resistor Pack (4x0603)   | 3.3kΩ                     | 1206 (4x0603)            | 2        | Front side of the board                                              |
| Resistor Pack (4x0603)   | 3.3kΩ                     | 1206 (4x0603)            | 2        | Back side of the board                                               |
| Ceramic Capacitor        | 0.1µF                     | 1206                     | 3        | Any for 5V, I used random 10V capacitor                              |
| Tantalum Capacitor       | 100µF                     | Case C (6032)            | 1        | Any for 3.3V, I used random 10V capacitor                            |
| MicroSD Card Slot        | -                         | SMD                      | 1        | [LSCS](https://www.lcsc.com/product-detail/C114218.html)             |
| Pi Pico 2040             | -                         | SMD                      | 1        |                                                                      |

## Known problems

### Rev 1

Because the Raspberry Pi Pico is positioned far from the edge of the board, not all micro-USB cables will fit.

When inserting the cable, hold it gently to prevent the SD card from popping out of the slot.