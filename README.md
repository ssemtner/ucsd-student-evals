# UCSD Student Evaluations API

This project is a high-performance data pipeline and RESTful API that provides access to UCSD's student evaluation of teaching (SET) data. It employs advanced web scraping techniques to gather comprehensive information about courses, instructors, and student evaluations, which is then stored in a PostgreSQL database and exposed through a secure and efficient API.

The data from this API is consumed by the [UCSD Student Evals Site](https://github.com/ssemtner/ucsd-student-evals-site), a user-friendly web interface for exploring and analyzing student evaluation data.

## Features

*   **Advanced Web Scraping:** Bypasses Duo-based SSO authentication by leveraging a private cookie server and a Duo instance running in an Android emulator.
*   **Efficient Data Collection:** Utilizes asynchronous programming with `tokio` and `futures` to concurrently scrape and process large volumes of data.
*   **Robust Data Storage:** Employs a PostgreSQL database with an optimized schema for efficient data storage and retrieval.
*   **High-Performance API:** A secure and scalable RESTful API built with `axum` to serve the collected data.
*   **Sophisticated SQL Queries:** Leverages advanced SQL features like `UNNEST` for bulk inserts, `ILIKE` for case-insensitive searching, and complex ordering with regex-based sorting.
*   **Comprehensive Data Coverage:** Scrapes and stores data for all courses, instructors, and evaluations available on the UCSD SET website.
*   **Containerized Deployment:** Deployed using Docker for easy and consistent deployment across different environments.

## How It Works

The project follows a multi-stage data pipeline to collect, store, and serve the student evaluation data.

### Web Scraping & Bypassing Authentication

The scraping process is designed to mimic a real user to bypass the security measures of the UCSD website.

1.  **Authentication:** The scraper authenticates with the UCSD website through a Duo-based SSO flow. This is achieved by using a private cookie server and a Duo instance running in an Android emulator, which provides the necessary authentication cookies.
2.  **Data Extraction:** Once authenticated, the scraper navigates the UCSD SET website to extract data about courses, instructors, and evaluations. It uses the `reqwest` library for making HTTP requests and `scraper` for parsing HTML.
3.  **Rate Limiting:** The scraper is designed to be mindful of the website's rate limits to avoid being blocked.

### Database Design

The collected data is stored in a PostgreSQL database with the following schema:

*   `courses`: Stores information about each course, including its code (e.g., "CSE 120") and name.
*   `units`: Contains the academic units (e.g., "CSE") that offer the courses.
*   `instructors`: A mapping of instructor IDs to their names, allowing for future expansion.
*   `terms`: A mapping of term IDs to their names (e.g., "Fall 2023").
*   `evaluations`: The main table containing the scraped evaluation data for each section, including student responses, grades, and hours spent.
*   `sids`: A table of section IDs (SIDs) that acts as a to-do list for the scraper. Any SID in this table that does not have a corresponding entry in the `evaluations` table is pending scraping.

### API

The `axum`-based API provides the following endpoints:

*   `GET /v1/courses`: Searches for courses with pagination support.
*   `GET /v1/evals/:code`: Retrieves a summary of evaluations for a specific course.
*   `GET /v1/evals/:code/instructors`: Lists the instructors who have taught a specific course.
*   `GET /v1/evals/:code/sections`: Lists all the section IDs for a given course.
*   `GET /v1/evals/sid/:sid`: Retrieves a summary for a specific section ID.

The API requires a private token for access, which is configured as an environment variable in the frontend application.

## Technologies Used

*   **Backend:** Rust, Tokio, Axum, SQLx
*   **Database:** PostgreSQL
*   **Web Scraping:** Reqwest, Scraper
*   **Deployment:** Docker

## Getting Started

To run this project locally, you will need to have Rust and PostgreSQL installed.

1.  **Clone the repository:**

    ```bash
    git clone https://github.com/ssemtner/ucsd-student-evals.git
    cd ucsd-student-evals
    ```

2.  **Set up the database:**

    *   Create a PostgreSQL database.
    *   Update the `database_url` in your `config.toml` file with your database connection string.

3.  **Install dependencies and run migrations:**

    ```bash
    cargo install sqlx-cli
    sqlx database create
    sqlx migrate run
    ```

4.  **Configure the application:**

    *   Copy the example configuration and update with your values:
    
    ```bash
    cp config.toml.example config.toml
    # Edit config.toml with your database URL, service URL, and API tokens
    ```

5.  **Run the application:**

    ```bash
    cargo run -- serve
    ```

## Deployment

The project is deployed on a private VPS using Docker with Traefik as a reverse proxy for automatic HTTPS with Let's Encrypt certificates. The `Dockerfile` and `docker-compose.yml` files in the repository can be used to build and run the application in a containerized environment. The `ucsd-student-evals-site` frontend is deployed on Vercel.

## Architecture Highlights

*   **Concurrent Processing**: Uses Rust's `tokio` async runtime with connection pooling for high-throughput data processing
*   **Type-Safe Database Operations**: Leverages `sqlx` with compile-time checked queries for robust database interactions
*   **Advanced Authentication**: Implements token-based API authentication with middleware for secure access control
*   **Scalable Design**: Modular architecture with separate modules for scraping, database operations, and API endpoints
*   **Production-Ready**: Includes proper error handling, logging with `tracing`, and structured configuration management