use chrono::{NaiveTime, Local};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Write};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Medication {
    name: String,
    time: NaiveTime,
    taken: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct MedicationSchedule {
    medications: HashMap<String, Vec<Medication>>, // Key: Date (YYYY-MM-DD)
}

impl MedicationSchedule {
    fn new() -> MedicationSchedule {
        MedicationSchedule {
            medications: HashMap::new(),
        }
    }

    fn add_medication(&mut self, date: String, medication: Medication) {
        self.medications
            .entry(date)
            .or_insert_with(Vec::new)
            .push(medication);
    }

    fn list_today(&self) -> Vec<Medication> {
        let today = Local::now().format("%Y-%m-%d").to_string();
        self.medications
            .get(&today)
            .cloned()
            .unwrap_or_else(Vec::new)
    }

    fn mark_taken(&mut self, date: &str, index: usize) -> Result<(), String> {
        if let Some(meds) = self.medications.get_mut(date) {
            if let Some(med) = meds.get_mut(index) {
                med.taken = true;
                Ok(())
            } else {
                Err("Invalid medication index".to_string())
            }
        } else {
            Err("No medications found for the date".to_string())
        }
    }

    fn export_missed_doses(&self, file_path: &str) -> io::Result<()> {
        let mut missed: Vec<Medication> = Vec::new();
        for (_date, meds) in &self.medications {
            for med in meds {
                if !med.taken && med.time < Local::now().time() {
                    missed.push(med.clone());
                }
            }
        }
        
        let file = File::create(file_path)?;
        let mut wtr = csv::Writer::from_writer(file);
        for med in missed {
            wtr.serialize(med)?;
        }
        wtr.flush()?;
        Ok(())
    }
}

fn main() {
    let mut schedule = MedicationSchedule::new();

    // Channel for communication between reminder thread and main thread
    let (tx, rx) = mpsc::channel();

    // Start the reminder thread
    thread::spawn(move || loop {
        let now = Local::now();
        let today = now.format("%Y-%m-%d").to_string();
        let current_time = now.time();

        // Send reminders for medications due now or earlier and not taken
        tx.send((today.clone(), current_time)).unwrap();

        // Check every minute
        thread::sleep(Duration::from_secs(60));
    });

    loop {
        // Check for reminders
        if let Ok((_today, current_time)) = rx.try_recv() {
            for med in schedule.list_today() {
                if med.time <= current_time && !med.taken {
                    println!(
                        "\nReminder: It's time to take your medication: {} at {}",
                        med.name,
                        med.time.format("%H:%M")
                    );
                }
            }
        }

        println!("\nMedication Reminder");
        println!("1. Add a medication");
        println!("2. View today's medication schedule");
        println!("3. Mark medication as taken");
        println!("4. Export missed doses");
        println!("5. Exit");

        print!("Choose an option: ");
        io::stdout().flush().unwrap();

        let mut choice = String::new();
        io::stdin().read_line(&mut choice).expect("Failed to read input");

        match choice.trim() {
            "1" => {
                let _today = Local::now().format("%Y-%m-%d").to_string();

                print!("Enter medication name: ");
                io::stdout().flush().unwrap();
                let mut name = String::new();
                io::stdin().read_line(&mut name).expect("Failed to read input");
                let name = name.trim().to_string();

                print!("Enter time (HH:MM): ");
                io::stdout().flush().unwrap();
                let mut time_input = String::new();
                io::stdin().read_line(&mut time_input).expect("Failed to read input");

                match NaiveTime::parse_from_str(time_input.trim(), "%H:%M") {
                    Ok(time) => {
                        let medication = Medication {
                            name,
                            time,
                            taken: false,
                        };
                        schedule.add_medication(_today, medication);
                        println!("Medication added successfully!");
                    }
                    Err(_) => {
                        println!("Invalid time format. Please use HH:MM.");
                    }
                }
            }
            "2" => {
                let today = Local::now().format("%Y-%m-%d").to_string();
                let meds = schedule.list_today();

                if meds.is_empty() {
                    println!("No medications scheduled for today.");
                } else {
                    println!("Today's Medication Schedule:");
                    for (i, med) in meds.iter().enumerate() {
                        println!(
                            "{}. {} at {} - {}",
                            i + 1,
                            med.name,
                            med.time.format("%H:%M"),
                            if med.taken { "Taken" } else { "Pending" }
                        );
                    }
                }
            }
            "3" => {
                let today = Local::now().format("%Y-%m-%d").to_string();
                let meds = schedule.list_today();

                if meds.is_empty() {
                    println!("No medications to mark as taken.");
                    continue;
                }

                println!("Select a medication to mark as taken:");
                for (i, med) in meds.iter().enumerate() {
                    println!(
                        "{}. {} at {} - {}",
                        i + 1,
                        med.name,
                        med.time.format("%H:%M"),
                        if med.taken { "Taken" } else { "Pending" }
                    );
                }

                print!("Enter the number: ");
                io::stdout().flush().unwrap();

                let mut index_input = String::new();
                io::stdin()
                    .read_line(&mut index_input)
                    .expect("Failed to read input");

                match index_input.trim().parse::<usize>() {
                    Ok(index) if index > 0 && index <= meds.len() => {
                        match schedule.mark_taken(&today, index - 1) {
                            Ok(_) => println!("Medication marked as taken!"),
                            Err(err) => println!("{}", err),
                        }
                    }
                    _ => println!("Invalid number."),
                }
            }
            "4" => {
                print!("Enter file name to export missed doses (e.g., missed.csv): ");
                io::stdout().flush().unwrap();
                let mut file_path = String::new();
                io::stdin().read_line(&mut file_path).expect("Failed to read input");

                match schedule.export_missed_doses(file_path.trim()) {
                    Ok(_) => println!("Missed doses exported successfully!"),
                    Err(err) => println!("Failed to export missed doses: {}", err),
                }
            }
            "5" => {
                println!("Goodbye!");
                break;
            }
            _ => {
                println!("Invalid choice. Please enter a number between 1 and 5.");
            }
        }
    }
}
