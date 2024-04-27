#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Booking {
    id: u64,
    customer_name: String,
    movie_name: String,
    showtime: String,
    num_tickets: u64,
}

// a trait that must be implemented for a struct that is stored in a stable struct
impl Storable for Booking {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

// another trait that must be implemented for a struct that is stored in a stable struct
impl BoundedStorable for Booking {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    static STORAGE: RefCell<StableBTreeMap<u64, Booking, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct BookingPayload {
    customer_name: String,
    movie_name: String,
    showtime: String,
    num_tickets: u64,
}

#[ic_cdk::query]
fn get_booking(id: u64) -> Result<Booking, Error> {
    match _get_booking(&id) {
        Some(booking) => Ok(booking),
        None => Err(Error::NotFound {
            msg: format!("a booking with id={} not found", id),
        }),
    }
}

#[ic_cdk::update]
fn add_booking(booking: BookingPayload) -> Option<Booking> {
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("cannot increment id counter");
    let booking = Booking {
        id,
        customer_name: booking.customer_name,
        movie_name: booking.movie_name,
        showtime: booking.showtime,
        num_tickets: booking.num_tickets,
    };
    do_insert(&booking);
    Some(booking)
}

#[ic_cdk::update]
fn update_booking(id: u64, payload: BookingPayload) -> Result<Booking, Error> {
    match STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut booking) => {
            booking.customer_name = payload.customer_name;
            booking.movie_name = payload.movie_name;
            booking.showtime = payload.showtime;
            booking.num_tickets = payload.num_tickets;
            do_insert(&booking);
            Ok(booking)
        }
        None => Err(Error::NotFound {
            msg: format!("couldn't update a booking with id={}. booking not found", id),
        }),
    }
}

// helper method to perform insert.
fn do_insert(booking: &Booking) {
    STORAGE.with(|service| service.borrow_mut().insert(booking.id, booking.clone()));
}

#[ic_cdk::update]
fn delete_booking(id: u64) -> Result<Booking, Error> {
    match STORAGE.with(|service| service.borrow_mut().remove(&id)) {
        Some(booking) => Ok(booking),
        None => Err(Error::NotFound {
            msg: format!("couldn't delete a booking with id={}. booking not found.", id),
        }),
    }
}

#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error {
    NotFound { msg: String },
}

// a helper method to get a booking by id. used in get_booking/update_booking
fn _get_booking(id: &u64) -> Option<Booking> {
    STORAGE.with(|service| service.borrow().get(id))
}

// need this to generate candid
ic_cdk::export_candid!();
