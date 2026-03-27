# rsm

rsm which stands for rust state machine, is a hierarchical state machine (hsm) implementation in rust. The development of this state machine framework was inspired by the [QuantumLeaps](https://www.state-machine.com/) state machine framework developed by Miro Samek. The quantum leaps website provides more resources for understanding hierarchical state machines than I could possibly cover here so I will derfer to the content on his webiste for explaining the concept.

This project was primarily conceived as a way for the author to learn rust. The author has used state machine frameworks in embedded development and has rolled their own before in C so thought it would be an instructive exercise to try and learn rust. 

If you've looked at other state machine frameworks in rust you'll notice that this framework is remarkably similar to [kaori-hsm](https://www.state-machine.com/). This is entirely by accident as I did not discover the framework until late in my development process. The underlying implementation is slightly different, but the API is almost identical. 
