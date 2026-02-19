# First board by @JDat for Raspberry Pi 2+

This is the first revision of the board, designed by @JDat (also known as @YL3AKC):

![Photo of prototype](../../photo/prototype.jpg)

In the photo, it's the old version with SRQ and ATN swapped.

## Components

| Part           | Value                | Package    | Quantity |
|----------------|----------------------|------------|----------|
| GPIB Connector | -                    | 90 Degree  | 1        |
| SN75160B       | -                    | SMD or DIP | 1        |
| SN75161B       | -                    | SMD or DIP | 1        |
| Resistor       | 3kΩ                  | 0805       | 20       |
| Capacitor      | 0.1uF                | 0805       | 2        |
| Pin Header     | 20x2 Straight Female | -          | 1        |


P.S. Instead of small SMD components, you can also use through-hole components, as Kirill @BOOtak Leyfer did.

https://t.me/bootaks_old_devices/286

## Errata

### Not enough space for the through-hole resistors.

As can be seen in Kirill's photo, the resistors are placed too close together, requiring them to be stacked on top of each other.
