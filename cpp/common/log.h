#ifndef LOG_H
#define LOG_H

#include <sstream>
#include <string>

#ifndef LOG_MODULE
#define LOG_MODULE "*"
#endif

namespace gol {
void error(const std::string &message);
void warn(const std::string &message);
void info(const std::string &message);
void debug(const std::string &message);
void trace(const std::string &message);
} // namespace gol

#define LOG_ERROR(message)                                                     \
  gol::error(std::string("[") + LOG_MODULE + "] " + message)
#define LOG_WARN(message)                                                      \
  gol::warn(std::string("[") + LOG_MODULE + "] " + message)
#define LOG_INFO(message)                                                      \
  gol::info(std::string("[") + LOG_MODULE + "] " + message)
#define LOG_DEBUG(message)                                                     \
  gol::debug(std::string("[") + LOG_MODULE + "] " + message)
#define LOG_TRACE(message)                                                     \
  gol::trace(std::string("[") + LOG_MODULE + "] " + message)

#ifdef _WIN32

#define HRB(f) MS_CHECK(f, return false;)
#define HRI(f) MS_CHECK(f, return -1;)
#define HRP(f) MS_CHECK(f, return nullptr;)
#define MS_CHECK(f, ...)                                                       \
  do {                                                                         \
    HRESULT __ms_hr__ = (f);                                                   \
    if (FAILED(__ms_hr__)) {                                                   \
      std::stringstream ss;                                                     \
      ss << "ERROR@" << __FILE__ << ":" << __LINE__ << " " << __FUNCTION__     \
         << " hr=0x" << std::hex << __ms_hr__ << std::dec << " "               \
         << std::error_code(__ms_hr__, std::system_category()).message();      \
      std::string result = ss.str();                                           \
      LOG_ERROR(result);                                                       \
      __VA_ARGS__                                                              \
    }                                                                          \
  } while (false)

#endif

#endif
