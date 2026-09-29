WITH MostActiveUsers AS (
    SELECT 
        u.Id, 
        u.DisplayName, 
        COUNT(p.Id) AS PostCount,
        SUM(v.BountyAmount) AS TotalBounty,
        SUM(u.UpVotes) AS TotalUpVotes,
        SUM(u.DownVotes) AS TotalDownVotes,
        ROW_NUMBER() OVER (ORDER BY COUNT(p.Id) DESC) AS UserRank
    FROM Users u
    LEFT JOIN Posts p ON u.Id = p.OwnerUserId
    LEFT JOIN Votes v ON u.Id = v.UserId
    WHERE u.Reputation > 1000
    GROUP BY u.Id, u.DisplayName
    HAVING COUNT(p.Id) > 5
),
CustomTagStats AS (
    SELECT 
        t.TagName, 
        COUNT(p.Id) AS PostCount,
        AVG(Length(p.Body)) AS AvgPostLength,
        MAX(p.CreationDate) AS LastPostDate
    FROM Tags t
    JOIN Posts p ON p.Tags LIKE '%' || t.TagName || '%'
    WHERE p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
    GROUP BY t.TagName
),
ClosedPostReasons AS (
    SELECT
        ph.UserId,
        ph.Comment,
        COUNT(ph.Id) AS CloseCount,
        SUM(epoch_us(cast('2024-10-01 12:34:56' as timestamp)) - epoch_us(ph.CreationDate))::DOUBLE / COUNT(ph.CreationDate) / 1e6 AS AvgCloseTime
    FROM PostHistory ph
    WHERE ph.PostHistoryTypeId = 10
    GROUP BY ph.UserId, ph.Comment
)
SELECT 
    aua.DisplayName,
    aua.PostCount,
    aua.TotalBounty,
    COALESCE(tas.PostCount, 0) AS TotalPostsForTags,
    COALESCE(tas.AvgPostLength, 0) AS AvgPostLength,
    COALESCE(cpr.CloseCount, 0) AS TotalCloseVotes,
    COALESCE(cpr.AvgCloseTime, 0) AS AvgTimeToClose
FROM MostActiveUsers aua
LEFT JOIN CustomTagStats tas ON tas.PostCount > 0
LEFT JOIN ClosedPostReasons cpr ON cpr.UserId = aua.Id
WHERE aua.UserRank <= 10
ORDER BY aua.TotalBounty DESC, aua.PostCount DESC;