WITH UserStats AS (
    SELECT 
        u.Id AS UserId,
        u.Reputation,
        COUNT(b.Id) AS BadgeCount,
        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
    FROM 
        Users u
    LEFT JOIN 
        Badges b ON u.Id = b.UserId
    LEFT JOIN 
        Votes v ON u.Id = v.UserId
    GROUP BY 
        u.Id, u.Reputation
),
PostStats AS (
    SELECT 
        p.OwnerUserId, 
        COUNT(p.Id) AS PostCount,
        SUM(COALESCE(p.Score, 0)) AS TotalScore,
        SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
        SUM(epoch_us(cast('2024-10-01 12:34:56' as timestamp)) - epoch_us(p.CreationDate))::DOUBLE / COUNT(p.CreationDate) / 1e6 / 60 AS AvgPostAgeInMinutes
    FROM 
        Posts p
    GROUP BY 
        p.OwnerUserId
),
Benchmark AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        us.Reputation,
        us.BadgeCount,
        us.TotalUpVotes,
        us.TotalDownVotes,
        ps.PostCount,
        ps.TotalScore,
        ps.TotalViews,
        ps.AvgPostAgeInMinutes
    FROM 
        UserStats us
    JOIN 
        PostStats ps ON us.UserId = ps.OwnerUserId
    JOIN 
        Users u ON us.UserId = u.Id
)

SELECT 
    UserId,
    DisplayName,
    Reputation,
    BadgeCount,
    TotalUpVotes,
    TotalDownVotes,
    PostCount,
    TotalScore,
    TotalViews,
    AvgPostAgeInMinutes
FROM 
    Benchmark
ORDER BY 
    Reputation DESC, PostCount DESC;