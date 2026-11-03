//! Outbound-port adapters that sit on top of the usecase.
//! Today only the `CrfServiceImpl` (which adapts `CrfUsecase`
//! to `apis::crf::CrfService`) lives here. The adapter is
//! generic over the repository ports, so the same impl serves
//! every storage backend.

pub mod internal;
