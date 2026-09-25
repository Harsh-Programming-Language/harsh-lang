from ipykernel.kernelapp import IPKernelApp
from .kernel import HarshKernel

IPKernelApp.launch_instance(kernel_class=HarshKernel)
