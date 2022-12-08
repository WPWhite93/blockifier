use std::collections::HashMap;
use std::mem;

use cairo_rs::types::relocatable::Relocatable;
use cairo_rs::vm::vm_core::VirtualMachine;
use starknet_api::hash::{StarkFelt, StarkHash};

use crate::execution::cairo_run_utils::{felt_to_bigint, get_felt_from_memory_cell};
use crate::execution::entry_point::EntryPointResult;
use crate::execution::syscall_handling::SyscallHandler;

pub type ReadRequestResult = EntryPointResult<Box<dyn SyscallRequest>>;
pub type ExecutionResult = EntryPointResult<Box<dyn SyscallResponse>>;
pub type WriteResponseResult = EntryPointResult<()>;

const STORAGE_READ_SELECTOR: &str = "0x53746f7261676552656164";
const STORAGE_WRITE_SELECTOR: &str = "0x53746f726167655772697465";

pub trait SyscallRequest {
    fn read(vm: &VirtualMachine, ptr: &Relocatable) -> ReadRequestResult
    where
        Self: Sized;

    fn execute(&self, syscall_handler: &SyscallHandler) -> ExecutionResult;
}

pub trait SyscallResponse {
    fn write(&self, vm: &mut VirtualMachine, ptr: &Relocatable) -> WriteResponseResult;
}

pub struct EmptyResponse {}

impl SyscallResponse for EmptyResponse {
    fn write(&self, _vm: &mut VirtualMachine, _ptr: &Relocatable) -> WriteResponseResult {
        Ok(())
    }
}

pub struct StorageReadRequest {
    pub selector: StarkFelt,
    pub address: StarkFelt,
}

impl SyscallRequest for StorageReadRequest {
    fn read(vm: &VirtualMachine, ptr: &Relocatable) -> ReadRequestResult {
        let selector = get_felt_from_memory_cell(vm.get_maybe(ptr)?)?;
        let address = get_felt_from_memory_cell(vm.get_maybe(&(ptr + 1))?)?;
        Ok(Box::new(StorageReadRequest { selector, address }))
    }

    fn execute(&self, _syscall_handler: &SyscallHandler) -> ExecutionResult {
        // TODO(AlonH, 21/12/2022): Perform state read.
        let value = StarkFelt::from(17);
        Ok(Box::new(StorageReadResponse { value }))
    }
}

pub struct StorageReadResponse {
    pub value: StarkFelt,
}

impl SyscallResponse for StorageReadResponse {
    fn write(&self, vm: &mut VirtualMachine, ptr: &Relocatable) -> WriteResponseResult {
        vm.insert_value(ptr, felt_to_bigint(self.value))?;
        Ok(())
    }
}

pub struct StorageRead {
    pub request: StorageReadRequest,
    pub response: StorageReadResponse,
}

pub struct StorageWriteRequest {
    pub selector: StarkFelt,
    pub address: StarkFelt,
    pub value: StarkFelt,
}

impl SyscallRequest for StorageWriteRequest {
    fn read(vm: &VirtualMachine, ptr: &Relocatable) -> ReadRequestResult {
        let selector = get_felt_from_memory_cell(vm.get_maybe(ptr)?)?;
        let address = get_felt_from_memory_cell(vm.get_maybe(&(ptr + 1))?)?;
        let value = get_felt_from_memory_cell(vm.get_maybe(&(ptr + 2))?)?;
        Ok(Box::new(StorageWriteRequest { selector, address, value }))
    }

    fn execute(&self, _syscall_handler: &SyscallHandler) -> ExecutionResult {
        // TODO(AlonH, 21/12/2022): Perform state write.
        assert_eq!(self.value, StarkFelt::try_from(18).unwrap());
        Ok(Box::new(EmptyResponse {}))
    }
}

pub struct StorageWrite {
    pub request: StorageWriteRequest,
    pub response: EmptyResponse,
}

pub type SyscallRequestFactory = dyn Fn(&VirtualMachine, &Relocatable) -> ReadRequestResult;

pub struct SyscallInfo {
    pub syscall_request_factory: Box<SyscallRequestFactory>,
    pub syscall_request_size: usize,
    pub syscall_response_size: usize,
}

// TODO(AlonH, 21/12/2022): Define and use a syscall selector enum instead of `StarkHash`.
pub fn get_syscall_info() -> HashMap<StarkHash, SyscallInfo> {
    let selector_error_msg = "Syscall selector should be able to turn into a `StarkHash`.";
    [
        (
            StarkHash::try_from(STORAGE_READ_SELECTOR).expect(selector_error_msg),
            SyscallInfo {
                syscall_request_factory: Box::new(StorageReadRequest::read),
                syscall_request_size: size_in_felts::<StorageReadRequest>(),
                syscall_response_size: size_in_felts::<StorageReadResponse>(),
            },
        ),
        (
            StarkHash::try_from(STORAGE_WRITE_SELECTOR).expect(selector_error_msg),
            SyscallInfo {
                syscall_request_factory: Box::new(StorageWriteRequest::read),
                syscall_request_size: size_in_felts::<StorageWriteRequest>(),
                syscall_response_size: size_in_felts::<EmptyResponse>(),
            },
        ),
    ]
    .into_iter()
    .collect()
}

pub fn size_in_felts<T>() -> usize {
    mem::size_of::<T>() / mem::size_of::<StarkFelt>()
}